use std::collections::HashMap;
use std::sync::RwLock;
use std::{env, fs};

use actix::Addr;
use actix_web::web;
use chrono::Utc;
use image::{ImageBuffer, Rgb};
use lettre::{transport::smtp, Transport};
use regex::Regex;
use thiserror::Error;

use crate::database::{Database, DatabaseUpdate};
use crate::models::user::User;
use crate::models::utils::{hex_to_rgb, ColorFile};
use crate::websocket::{MessageUpdate, PlaceWebSocketConnection};

#[derive(Error, Debug)]
pub enum AppStateError {
    #[error("Error getting pixels")]
    PixelFetchError(String),
    #[error("Error getting users")]
    UserFetchError,
    #[error("SMTP configuration error")]
    SmtpConfigError,
    #[error("Regex compilation error")]
    RegexCompileError,
    #[error("File read error: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("JSON deserialization error: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("Environment variable not set: {0}")]
    EnvVarNotSet(String),
    #[error("Invalid value: {0}")]
    InvalidValueError(String),
    #[error("Error parsing email")]
    EmailParseError,
    #[error("Error creating verification email")]
    EmailCreationError,
    #[error("Error sending verification email")]
    EmailSendingError,
    #[error("Error adding session")]
    SessionAddError,
    #[error("No such user")]
    NoSuchUserError,
}

pub struct AppState {
    width: usize,
    height: usize,
    pixels_color: Vec<u8>,
    pixels_user: Vec<u16>,
    palette: Vec<(u8, u8, u8)>,
    users: HashMap<u16, User>,
    png: Vec<u8>,
    last_update: i64,
    update_cooldown: u16,
    database_updates: Vec<DatabaseUpdate>,
    message_updates: Vec<MessageUpdate>,
    mailer: lettre::SmtpTransport,
    sessions: RwLock<Vec<Addr<PlaceWebSocketConnection>>>,
    email_regex: Regex,
    cooldown: u16,
    jwt_secret: String,
    smtp_user: String,
    url: String,
}

impl AppState {
    pub fn new(width: usize, height: usize, db: &Database) -> Result<Self, AppStateError> {
        let (pixels_color, pixels_user) = db
            .get_pixels(width, height)
            .map_err(|e| AppStateError::PixelFetchError(e.to_string()))?;

        let users = db.get_users().map_err(|_| AppStateError::UserFetchError)?;

        let smtp_server = env::var("SMTP_SERVER")
            .map_err(|_| AppStateError::EnvVarNotSet("SMTP_SERVER".to_string()))?;
        let smtp_port: u16 = env::var("SMTP_PORT")
            .map_err(|_| AppStateError::EnvVarNotSet("SMTP_PORT".to_string()))?
            .parse()
            .map_err(|_| AppStateError::InvalidValueError("SMTP_PORT".to_string()))?;
        let smtp_user = env::var("SMTP_USER")
            .map_err(|_| AppStateError::EnvVarNotSet("SMTP_USER".to_string()))?;
        let smtp_password = env::var("SMTP_PASSWORD")
            .map_err(|_| AppStateError::EnvVarNotSet("SMTP_PASSWORD".to_string()))?;

        let mailer = lettre::SmtpTransport::starttls_relay(&smtp_server)
            .map_err(|_| AppStateError::SmtpConfigError)?
            .port(smtp_port)
            .credentials(smtp::authentication::Credentials::new(
                smtp_user.clone(),
                smtp_password,
            ))
            .build();

        let email_regex =
            Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$")
                .map_err(|_| AppStateError::RegexCompileError)?;

        let cooldown = env::var("COOLDOWN_SEC")
            .map_err(|_| AppStateError::EnvVarNotSet("COOLDOWN_SEC".to_string()))?
            .parse::<u16>()
            .map_err(|_| AppStateError::InvalidValueError("COOLDOWN".to_string()))?;

        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| AppStateError::EnvVarNotSet("JWT_SECRET".to_string()))?;

        let update_cooldown = env::var("UPDATE_COOLDOWN_SEC")
            .map_err(|_| AppStateError::EnvVarNotSet("UPDATE_COOLDOWN_SEC".to_string()))?
            .parse::<u16>()
            .map_err(|_| AppStateError::InvalidValueError("UPDATE_COOLDOWN_SEC".to_string()))?;

        let colors_path = env::var("COLORS_PATH")
            .map_err(|_| AppStateError::EnvVarNotSet("COLORS_PATH".to_string()))?;
        let colors_str = fs::read_to_string(colors_path).map_err(AppStateError::FileReadError)?;
        let color_file = serde_json::from_str::<ColorFile>(&colors_str)
            .map_err(AppStateError::JsonParseError)?;
        let palette: Vec<(u8, u8, u8)> = color_file
            .colors
            .iter()
            .map(|color| hex_to_rgb(color))
            .collect();

        let url = env::var("URL").map_err(|_| AppStateError::EnvVarNotSet("URL".to_string()))?;

        Ok(Self {
            width,
            height,
            pixels_color,
            pixels_user,
            palette,
            users,
            last_update: 0,
            update_cooldown,
            database_updates: Vec::new(),
            message_updates: Vec::new(),
            mailer,
            sessions: RwLock::new(Vec::new()),
            email_regex,
            cooldown,
            jwt_secret,
            png: Vec::new(),
            smtp_user,
            url,
        })
    }

    pub fn draw(
        &mut self,
        x: usize,
        y: usize,
        user_id: u16,
        color: u8,
    ) -> Result<(), AppStateError> {
        if x >= self.width || y >= self.height {
            return Err(AppStateError::InvalidValueError(
                "x or y out of bounds".to_string(),
            ));
        }

        let index = x * self.height + y;
        self.pixels_user[index] = user_id;
        self.pixels_color[index] = color;

        let user = self
            .users
            .get_mut(&user_id)
            .ok_or_else(|| AppStateError::NoSuchUserError)?;
        user.cooldown = Utc::now().timestamp() + self.cooldown as i64;
        user.score += 1;

        self.database_updates.push(DatabaseUpdate {
            x,
            y,
            color,
            user_id,
            timestamp: Utc::now().timestamp(),
        });
        let message_update = MessageUpdate { x, y, color };
        self.message_updates.push(message_update);
        self.broadcast(message_update)?;

        Ok(())
    }

    pub fn send_verification_mail(&self, email: &str, token: &str) -> Result<(), AppStateError> {
        let parsed_from = self
            .smtp_user
            .parse()
            .map_err(|_| AppStateError::EmailParseError)?;
        let parsed_to = email.parse().map_err(|_| AppStateError::EmailParseError)?;

        let email_body = format!(
            "Click on this link to verify your account: {}/api/verify/{}",
            self.url, token,
        );

        let email = lettre::Message::builder()
            .from(parsed_from)
            .to(parsed_to)
            .subject("Verify your account")
            .body(email_body)
            .map_err(|_| AppStateError::EmailCreationError)?;

        self.mailer
            .send(&email)
            .map_err(|_| AppStateError::EmailSendingError)?;
        Ok(())
    }

    pub fn add_session(
        &self,
        session: Addr<PlaceWebSocketConnection>,
    ) -> Result<(), AppStateError> {
        self.sessions
            .write()
            .map(|mut sessions| sessions.push(session))
            .map_err(|_| AppStateError::SessionAddError)
    }

    fn broadcast(&self, msg: MessageUpdate) -> Result<(), AppStateError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| AppStateError::SessionAddError)?;
        for session in sessions.iter() {
            session.do_send(msg);
        }
        Ok(())
    }

    pub fn try_update(&mut self, db: &web::Data<Database>) -> Result<(), AppStateError> {
        let time = Utc::now().timestamp();
        if time - self.last_update < self.update_cooldown as i64 {
            return Ok(());
        }
        self.last_update = time;

        let image = ImageBuffer::from_fn(self.width as u32, self.height as u32, |x, y| {
            let index = (x as usize) * self.height + (y as usize);
            let color = self.palette[self.pixels_color[index] as usize];
            Rgb([color.0, color.1, color.2])
        });

        let mut new_png: Vec<u8> = Vec::new();
        {
            let mut cursor = std::io::Cursor::new(&mut new_png);
            image
                .write_to(&mut cursor, image::ImageOutputFormat::Png)
                .map_err(|_| {
                    AppStateError::FileReadError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Error writing image",
                    ))
                })?;
        }

        self.png = new_png;

        db.save_pixel_updates(&self.database_updates).map_err(|e| {
            eprintln!("Error saving pixel updates: {}", e);
            AppStateError::PixelFetchError(e.to_string())
        })?;

        let mut users: Vec<&mut User> = self.users.values_mut().collect();
        users.sort_by(|a, b| b.score.cmp(&a.score));
        for (rank, user) in users.iter_mut().enumerate() {
            user.rank = rank as u32 + 1;
        }

        self.database_updates.clear();
        self.message_updates.clear();

        Ok(())
    }

    pub fn get_size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn get_png(&self) -> &Vec<u8> {
        &self.png
    }

    pub fn get_message_updates(&self) -> Vec<MessageUpdate> {
        self.message_updates.clone()
    }

    pub fn get_user(&self, id: u16) -> Option<&User> {
        self.users.get(&id)
    }

    pub fn get_user_mut(&mut self, id: u16) -> Option<&mut User> {
        self.users.get_mut(&id)
    }

    pub fn insert_user(&mut self, id: u16, user: User) {
        self.users.insert(id, user);
    }

    pub fn get_leaderboard(&self) -> Vec<User> {
        let mut users: Vec<User> = self.users.values().cloned().collect();
        users.sort_by(|a, b| a.rank.cmp(&b.rank));
        users.into_iter().take(10).collect()
    }

    pub fn is_username_taken(&self, username: &str) -> bool {
        self.users.values().any(|user| user.username == username)
    }

    pub fn user_length(&self) -> usize {
        self.users.len()
    }

    pub fn get_users_connected(&self) -> usize {
        self.sessions.read().unwrap().len()
    }

    pub fn get_username_from_pixel(&self, x: usize, y: usize) -> String {
        let index = x * self.height + y;
        let user_id = self.pixels_user[index];
        let username = match self.users.get(&user_id) {
            Some(user) => user.username.clone(),
            None => "No username".to_string(),
        };
        username
    }

    pub fn email_regex(&self) -> &Regex {
        &self.email_regex
    }

    pub fn cooldown(&self) -> u16 {
        self.cooldown
    }

    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

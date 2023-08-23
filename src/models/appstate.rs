use std::collections::HashMap;
use std::sync::RwLock;
use std::{env, fs};

use actix::Addr;
use chrono::Utc;
use image::{ImageBuffer, Rgb};
use lettre::{transport::smtp, Transport};
use regex::Regex;

use crate::database;
use crate::database::DatabaseUpdate;
use crate::models::user::User;
use crate::models::utils::{hex_to_rgb, ColorFile};
use crate::websocket::{MessageUpdate, PlaceWebSocketConnection};

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
    ubs_regex: Regex,
    cooldown: u16,
    jwt_secret: String,
}

impl AppState {
    pub fn new(width: usize, height: usize) -> Self {
        let db = database::Database::new().expect("Error connecting to database");

        let (pixels_color, pixels_user) =
            db.get_pixels(width, height).expect("Error getting pixels");

        let users = db.get_users().expect("Error getting users");

        let smtp_server = env::var("SMTP_SERVER").expect("SMTP_SERVER must be set");
        let smtp_port: u16 = env::var("SMTP_PORT")
            .expect("SMTP_PORT must be set")
            .parse()
            .expect("SMTP_PORT should be a valid u16");
        let smtp_user = env::var("SMTP_USER").expect("SMTP_USER must be set");
        let smtp_password = env::var("SMTP_PASSWORD").expect("SMTP_PASSWORD must be set");

        let mailer = lettre::SmtpTransport::starttls_relay(&smtp_server)
            .expect("Error starting mail relay")
            .port(smtp_port)
            .credentials(smtp::authentication::Credentials::new(
                smtp_user,
                smtp_password,
            ))
            .build();

        let email_regex =
            Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$")
                .expect("Error compiling email regex");

        let ubs_regex =
            Regex::new(r"^[a-z0-9.]+@(etud\.)?univ-ubs\.fr$").expect("Error compiling ubs regex");

        let cooldown = env::var("COOLDOWN_SEC")
            .expect("COOLDOWN must be set")
            .parse::<u16>()
            .expect("COOLDOWN should be a valid u16");

        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

        let update_cooldown = env::var("UPDATE_COOLDOWN_SEC")
            .expect("UPDATE_COOLDOWN_SEC must be set")
            .parse::<u16>()
            .expect("UPDATE_COOLDOWN_SEC should be a valid u16");

        let colors_str =
            fs::read_to_string("public/misc/colors.json").expect("Error reading colors file");
        let color_file =
            serde_json::from_str::<ColorFile>(&colors_str).expect("Error parsing colors file");
        let palette: Vec<(u8, u8, u8)> = color_file
            .colors
            .iter()
            .map(|color| hex_to_rgb(color))
            .collect();

        Self {
            width,
            height,
            pixels_color,
            pixels_user,
            users,
            last_update: 0,
            update_cooldown,
            database_updates: Vec::new(),
            message_updates: Vec::new(),
            mailer,
            sessions: RwLock::new(Vec::new()),
            email_regex,
            ubs_regex,
            cooldown,
            jwt_secret,
            png: Vec::new(),
            palette,
        }
    }

    pub fn draw(&mut self, x: usize, y: usize, user_id: u16, color: u8) {
        let index = x * self.height + y;
        self.pixels_user[index] = user_id;
        self.pixels_color[index] = color;
        self.users.get_mut(&user_id).unwrap().score += 1;
        self.database_updates.push(DatabaseUpdate {
            x,
            y,
            color,
            user_id,
            timestamp: Utc::now().timestamp(),
        });
        let message_update = MessageUpdate { x, y, color };
        self.message_updates.push(message_update);
        self.broadcast(message_update);
    }

    pub fn send_verification_mail(&self, email: &str, token: &str) {
        let from_address = env::var("SMTP_USER").expect("SMTP_USER must be set");

        let url = env::var("URL").expect("URL must be set");

        let from_address = from_address.parse();
        let to_address = email.parse();

        if let (Ok(from_address), Ok(to_address)) = (from_address, to_address) {
            let email_body = format!(
                "Click on this link to verify your account: {}/api/verify/{}",
                url, token,
            );
            let email = lettre::Message::builder()
                .from(from_address)
                .to(to_address)
                .subject("Verify your account")
                .body(email_body);

            match email {
                Ok(email) => {
                    if let Err(err) = self.mailer.send(&email) {
                        println!("Error sending verification email: {}", err);
                    }
                }
                Err(err) => {
                    println!("Error creating verification email: {}", err);
                }
            };
        } else {
            println!("Error parsing email addresses");
        }
    }

    pub fn add_session(&self, session: Addr<PlaceWebSocketConnection>) {
        match self.sessions.write() {
            Ok(mut sessions) => {
                sessions.push(session);
            }
            Err(err) => {
                println!("Error adding session: {}", err);
            }
        }
    }

    fn broadcast(&self, msg: MessageUpdate) {
        let sessions = match self.sessions.read() {
            Ok(sessions) => sessions,
            Err(err) => {
                println!("Error reading sessions for broadcast: {}", err);
                return;
            }
        };
        for session in sessions.iter() {
            session.do_send(msg);
        }
    }

    pub fn try_update(&mut self) -> Result<(), String> {
        let time = Utc::now().timestamp();
        if time - self.last_update < self.update_cooldown as i64 {
            return Ok(());
        }

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
                .map_err(|err| format!("Error writing image: {}", err))?;
        }

        self.png = new_png;

        let mut db = database::Database::new()
            .map_err(|err| format!("Error connecting to database: {}", err))?;

        match db.save_pixel_updates(&self.database_updates) {
            Ok(_) => (),
            Err(err) => eprintln!("Error saving pixel updates: {}", err),
        }

        let mut users: Vec<&mut User> = self.users.values_mut().collect();
        users.sort_by(|a, b| b.score.cmp(&a.score));
        for (rank, user) in users.iter_mut().enumerate() {
            user.rank = rank as u32 + 1;
        }

        self.database_updates.clear();
        self.message_updates.clear();
        self.last_update = time;

        Ok(())
    }

    pub fn get_size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn get_png(&self) -> Vec<u8> {
        self.png.clone()
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

    pub fn ubs_regex(&self) -> &Regex {
        &self.ubs_regex
    }

    pub fn cooldown(&self) -> u16 {
        self.cooldown
    }

    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

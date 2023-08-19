use std::env;
use std::io::Write;
use std::sync::RwLock;
use actix::{Addr, Message};
use chrono::Utc;
use flate2::Compression;
use flate2::write::GzEncoder;
use lettre::Transport;
use lettre::transport::smtp;
use serde_derive::Serialize;
use crate::database;
use crate::websocket::PlaceWebSocketConnection;

#[derive(Message, Clone, Copy, Serialize)]
#[rtype(result = "()")]
pub struct UpdateMessage{
    pub x: usize,
    pub y: usize,
    pub color: u8,
}

pub struct AppState {
    width: usize,
    height: usize,
    pixels_color: Vec<u8>,
    pixels_user: Vec<String>,
    compressed_pixels_color: Vec<u8>,
    last_update: i64,
    mailer: lettre::SmtpTransport,
    sessions: RwLock<Vec<Addr<PlaceWebSocketConnection>>>,
}

impl AppState {
    pub fn new(width: usize, height: usize) -> Self {
        let db = database::Database::new()
            .expect("Error connecting to database");

        let pixels_color = db.get_pixel_grid(width, height)
            .expect("Error getting pixel grid");

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
            .credentials(
                smtp::authentication::Credentials::new(
                    smtp_user,
                    smtp_password,
                )
            )
            .build();

        Self {
            width,
            height,
            pixels_color,
            pixels_user: vec![String::new(); width * height],
            compressed_pixels_color: Vec::new(),
            last_update: 0,
            mailer,
            sessions: RwLock::new(Vec::new()),
        }
    }

    pub fn get_pixels_color(&mut self) -> Vec<u8> {
        let time = Utc::now().timestamp_millis();
        if time - self.last_update > 1000 {
            self.last_update = time;
            self.compressed_pixels_color = match Self::compress(&self.pixels_color) {
                Ok(compressed_pixels_color) => compressed_pixels_color,
                Err(err) => {
                    println!("Error compressing pixels: {}", err);
                    return self.compressed_pixels_color.clone();
                }
            };
        }
        self.compressed_pixels_color.clone()
    }

    pub fn draw(&mut self, x: usize, y: usize, user: &str, color: u8) {
        let index = x * self.height + y;
        self.pixels_user[index] = user.to_string();
        self.pixels_color[index] = color;
        self.broadcast(UpdateMessage { x, y, color });
    }

    fn compress(grid: &Vec<u8>) -> Result<Vec<u8>, std::io::Error> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(grid)?;
        Ok(encoder.finish()?)
    }

    pub fn send_verification_mail(&self, email: &str, token: &str) {
        let from_address = "place.vannes.verif@outlook.com".parse();
        let to_address = email.parse();

        if let (Ok(from_address), Ok(to_address)) = (from_address, to_address) {
            let email_body = format!(
                "Click on this link to verify your account: http://localhost:8080/api/verify/{}",
                token
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

    fn broadcast(&self, update_message: UpdateMessage) {
        let sessions = match self.sessions.read() {
            Ok(sessions) => sessions,
            Err(err) => {
                println!("Error reading sessions for broadcast: {}", err);
                return;
            }
        };
        for session in sessions.iter() {
            session.do_send(update_message);
        }
    }

    pub fn get_size(&self) -> (usize, usize) {
        (self.width, self.height)
    }
}

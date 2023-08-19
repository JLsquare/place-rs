use std::env;
use chrono::Utc;
use rusqlite::params;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error("Drawing too fast")]
    DrawingTooFast,
    #[error("Invalid token")]
    InvalidToken,
}

pub struct Database {
    connection: rusqlite::Connection,
}

impl Database {
    pub fn new() -> Result<Self, DatabaseError> {
        let db_url: String = env::var("DB_URL")
            .expect("DB_URL environment variable not set");

        let connection = rusqlite::Connection::open(db_url)?;
        Ok(Self { connection })
    }

    pub fn create_tables(&self) -> Result<(), DatabaseError> {
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS users (
                username TEXT PRIMARY KEY,
                password TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                pixelCount INTEGER NOT NULL DEFAULT 0,
                verificationCode TEXT NOT NULL,
                verified INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS pixels (
                x INTEGER NOT NULL,
                y INTEGER NOT NULL,
                color INTEGER NOT NULL,
                user TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                PRIMARY KEY (x, y, timestamp)
            )",
            [],
        )?;

        self.connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_user ON pixels(user)",
            [],
        )?;

        self.connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_timestamp ON pixels(user, timestamp DESC)",
            [],
        )?;

        Ok(())
    }

    pub fn signup(&self, username: &str, password: &str, email: &str, verification_code: &str) -> Result<(), DatabaseError> {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        self.connection.execute(
            "INSERT INTO users (username, password, email, verificationCode) VALUES (?1, ?2, ?3, ?4)",
            params![username, &hash, email, verification_code],
        )?;
        Ok(())
    }

    pub fn verify(&self, verification_code: &str) -> Result<(), DatabaseError> {
        let mut statement = self.connection.prepare("SELECT username FROM users WHERE verificationCode = ?1")?;
        let mut rows = statement.query(params![verification_code])?;
        if let Some(row) = rows.next()? {
            let username: String = row.get(0)?;
            self.connection.execute(
                "UPDATE users SET verified = 1 WHERE username = ?1",
                params![username],
            )?;
            Ok(())
        } else {
            Err(DatabaseError::InvalidToken)
        }
    }

    pub fn login(&self, username: &str, password: &str) -> Result<bool, DatabaseError> {
        let mut statement = self.connection.prepare("SELECT password FROM users WHERE username = ?1")?;
        let mut rows = statement.query(params![username])?;

        if let Some(row) = rows.next()? {
            let hash: String = row.get(0)?;
            Ok(bcrypt::verify(password, &hash)?)
        } else {
            Ok(false)
        }
    }

    pub fn draw(&self, x: u32, y: u32, color: u8, user: &str) -> Result<(), DatabaseError> {
        let time = Utc::now().timestamp();
        let mut statement = self.connection.prepare(
            "SELECT timestamp FROM pixels WHERE user = ?1 ORDER BY timestamp DESC LIMIT 1",
        )?;
        let mut rows = statement.query(params![user])?;
        let db_time: i64 = if let Some(row) = rows.next()? {
            row.get(0)?
        } else {
            0
        };
        if time - db_time < 5 {
            return Err(DatabaseError::DrawingTooFast);
        }

        self.connection.execute(
            "INSERT INTO pixels (x, y, color, user, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![x as i64, y as i64, color as i64, user, time],
        )?;

        self.connection.execute(
            "UPDATE users SET pixelCount = pixelCount + 1 WHERE username = ?1",
            params![user],
        )?;

        Ok(())
    }

    pub fn get_leaderboard(&self) -> Result<Vec<(String, i64)>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT username, pixelCount FROM users ORDER BY pixelCount DESC LIMIT 10",
        )?;
        let mut rows = statement.query([])?;

        let mut leaderboard = Vec::new();
        while let Some(row) = rows.next()? {
            leaderboard.push((row.get(0)?, row.get(1)?));
        }

        Ok(leaderboard)
    }

    pub fn get_pixel_grid(&self, width: usize, height: usize) -> Result <Vec<u8>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT pixels.x, pixels.y, pixels.color
            FROM pixels
            JOIN (
                SELECT x, y, MAX(timestamp) as max_timestamp
                FROM pixels
                GROUP BY x, y
            ) pixel
            ON pixels.x = pixel.x AND pixels.y = pixel.y AND pixels.timestamp = pixel.max_timestamp
            ORDER BY pixels.x, pixels.y;",
        )?;
        let mut rows = statement.query([])?;

        let mut pixels = vec![31; width * height];
        while let Some(row) = rows.next()? {
            let x: i64 = row.get(0)?;
            let y: i64 = row.get(1)?;
            let color: i64 = row.get(2)?;
            pixels[(x * height as i64 + y) as usize] = color as u8;
        }

        Ok(pixels)
    }

    pub fn get_cooldown(&self, user: &str) -> Result<i64, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT timestamp FROM pixels WHERE user = ?1 ORDER BY timestamp DESC LIMIT 1",
        )?;
        let mut rows = statement.query(params![user])?;
        if let Some(row) = rows.next()? {
            let db_time: i64 = row.get(0)?;
            Ok(db_time)
        } else {
            Ok(0)
        }
    }
}

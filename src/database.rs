use std::collections::HashMap;
use std::env;

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use thiserror::Error;

use crate::models::user::User;
use crate::routes::user::ProfileEdit;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error(transparent)]
    R2d2(#[from] r2d2::Error),
    #[error("No such row")]
    NoSuchRow,
}

pub struct DatabaseUpdate {
    pub x: usize,
    pub y: usize,
    pub color: u8,
    pub user_id: u16,
    pub timestamp: i64,
}

pub struct Database {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn new() -> Result<Self, DatabaseError> {
        let db_url: String = env::var("DB_URL").expect("DB_URL environment variable not set");
        let manager = SqliteConnectionManager::file(db_url);
        let pool = r2d2::Pool::new(manager)?;
        Ok(Self { pool })
    }

    pub fn create_tables(&self) -> Result<(), DatabaseError> {
        let connection = self.pool.get()?;

        connection.execute("PRAGMA foreign_keys = ON;", params![])?;

        connection.execute(
            "CREATE TABLE IF NOT EXISTS users (
                user_id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                verification_code TEXT NOT NULL,
                verified INTEGER NOT NULL DEFAULT 0,
                ubs_id INTEGER NOT NULL
            )",
            [],
        )?;

        connection.execute(
            "CREATE TABLE IF NOT EXISTS pixels (
                pixel_id INTEGER PRIMARY KEY AUTOINCREMENT,
                x INTEGER NOT NULL,
                y INTEGER NOT NULL,
                color INTEGER NOT NULL,
                user INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                UNIQUE(x, y, timestamp),
                FOREIGN KEY(user) REFERENCES users(user_id)
            )",
            [],
        )?;

        Ok(())
    }

    pub fn signup(
        &self,
        username: &str,
        password: &str,
        email: &str,
        verification_code: &str,
        ubs_id: u32,
    ) -> Result<u16, DatabaseError> {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        let connection = self.pool.get()?;

        connection.execute(
            "INSERT INTO users (username, password, email, verification_code, ubs_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![username, &hash, email, verification_code, ubs_id],
        )?;

        Ok(connection.last_insert_rowid() as u16)
    }

    pub fn check_ubs_id(&self, ubs_id: u32) -> Result<bool, DatabaseError> {
        let connection = self.pool.get()?;

        let mut statement = connection.prepare("SELECT user_id FROM users WHERE ubs_id = ?1")?;
        let mut rows = statement.query(params![ubs_id])?;
        if rows.next()?.is_some() {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn verify(&self, verification_code: &str) -> Result<u16, DatabaseError> {
        let connection = self.pool.get()?;

        let mut statement =
            connection.prepare("SELECT user_id FROM users WHERE verification_code = ?1")?;
        let mut rows = statement.query(params![verification_code])?;
        if let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            connection.execute(
                "UPDATE users SET verified = 1 WHERE user_id = ?1",
                params![id],
            )?;
            Ok(id as u16)
        } else {
            Err(DatabaseError::NoSuchRow)
        }
    }

    pub fn login(&self, username: &str, password: &str) -> Result<Option<u16>, DatabaseError> {
        let connection = self.pool.get()?;

        let mut statement =
            connection.prepare("SELECT user_id, password FROM users WHERE username = ?1")?;
        let mut rows = statement.query(params![username])?;

        if let Some(row) = rows.next()? {
            let user_id: i64 = row.get(0)?;
            let hash: String = row.get(1)?;

            if bcrypt::verify(password, &hash)? {
                Ok(Some(user_id as u16))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub fn check_password(&self, user_id: u16, password: &str) -> Result<bool, DatabaseError> {
        let connection = self.pool.get()?;

        let mut statement = connection.prepare("SELECT password FROM users WHERE user_id = ?1")?;
        let mut rows = statement.query(params![user_id])?;

        if let Some(row) = rows.next()? {
            let hash: String = row.get(0)?;

            if bcrypt::verify(password, &hash)? {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    pub fn get_pixels(
        &self,
        width: usize,
        height: usize,
    ) -> Result<(Vec<u8>, Vec<u16>), DatabaseError> {
        let connection = self.pool.get()?;
        let mut statement = connection.prepare(
            "SELECT pixels.x, pixels.y, pixels.user, pixels.color
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
        let mut pixels_color = vec![31; width * height];
        let mut pixels_user = vec![0; width * height];
        while let Some(row) = rows.next()? {
            let x: i64 = row.get(0)?;
            let y: i64 = row.get(1)?;
            let user: i64 = row.get(2)?;
            let color: i64 = row.get(3)?;
            pixels_color[(x * height as i64 + y) as usize] = color as u8;
            pixels_user[(x * height as i64 + y) as usize] = user as u16;
        }

        Ok((pixels_color, pixels_user))
    }

    pub fn get_users(&self) -> Result<HashMap<u16, User>, DatabaseError> {
        let connection = self.pool.get()?;

        let mut statement = connection.prepare(
            "SELECT user_id, username, COUNT(*) as pixel_count, verified
            FROM users
            JOIN pixels
            ON users.user_id = pixels.user
            GROUP BY users.user_id",
        )?;
        let mut rows = statement.query([])?;

        let mut users = HashMap::new();
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let username: String = row.get(1)?;
            let pixel_count: i64 = row.get(2)?;
            let verified: i64 = row.get(3)?;
            users.insert(
                id as u16,
                User {
                    username,
                    cooldown: 0,
                    rank: 0,
                    verified: verified == 1,
                    score: pixel_count as u32,
                },
            );
        }

        Ok(users)
    }

    pub fn edit_profile(&self, user: u16, profile_edit: &ProfileEdit) -> Result<(), DatabaseError> {
        let connection = self.pool.get()?;

        if profile_edit.password.trim().is_empty() {
            let mut statement = connection.prepare(
                "\
                UPDATE users SET username = ?1 WHERE user_id = ?2",
            )?;
            statement.execute(params![profile_edit.username, user])?;
        } else {
            let hash = bcrypt::hash(&profile_edit.password, bcrypt::DEFAULT_COST)?;
            let mut statement = connection.prepare(
                "\
                UPDATE users SET username = ?1, password = ?2 WHERE user_id = ?3",
            )?;
            statement.execute(params![profile_edit.username, hash, user])?;
        }

        Ok(())
    }

    pub fn save_pixel_updates(
        &mut self,
        updates: &Vec<DatabaseUpdate>,
    ) -> Result<(), DatabaseError> {
        let mut connection = self.pool.get()?;

        let tx = connection.transaction()?;
        {
            let mut statement = tx.prepare(
                "INSERT INTO pixels (x, y, color, user, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;

            for update in updates {
                statement.execute(params![
                    update.x as i64,
                    update.y as i64,
                    update.color as i64,
                    update.user_id,
                    update.timestamp
                ])?;
            }
        }
        tx.commit()?;

        Ok(())
    }
}

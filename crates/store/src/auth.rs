use std::sync::Mutex;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use chrono::{DateTime, Duration, TimeZone, Utc};
use rand::RngCore;
use rusqlite::{params, Connection};

use crate::sqlite::StoreError;

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
}

const SESSION_TTL_HOURS: i64 = 24 * 7;

pub trait AuthStore: Send + Sync {
    fn user_count(&self) -> Result<i64, StoreError>;
    fn create_user(&self, username: &str, password: &str) -> Result<User, StoreError>;
    fn verify_login(&self, username: &str, password: &str) -> Result<Option<User>, StoreError>;
    fn create_session(&self, user_id: i64) -> Result<String, StoreError>;
    fn validate_session(&self, token: &str) -> Result<Option<User>, StoreError>;
    fn delete_session(&self, token: &str) -> Result<(), StoreError>;
}

pub struct SqliteAuthStore {
    conn: Mutex<Connection>,
}

fn hash_password(password: &str) -> Result<String, StoreError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| StoreError::Auth(e.to_string()))
}

fn verify_password(password: &str, hash: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(hash) else { return false };
    Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok()
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl SqliteAuthStore {
    pub fn open(path: &str) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at_unix_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                created_at_unix_ms INTEGER NOT NULL,
                expires_at_unix_ms INTEGER NOT NULL
            );",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }
}

impl AuthStore for SqliteAuthStore {
    fn user_count(&self) -> Result<i64, StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0)).map_err(Into::into)
    }

    fn create_user(&self, username: &str, password: &str) -> Result<User, StoreError> {
        let hash = hash_password(password)?;
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        conn.execute(
            "INSERT INTO users (username, password_hash, created_at_unix_ms) VALUES (?1, ?2, ?3)",
            params![username, hash, Utc::now().timestamp_millis()],
        )?;
        Ok(User { id: conn.last_insert_rowid(), username: username.to_string() })
    }

    fn verify_login(&self, username: &str, password: &str) -> Result<Option<User>, StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        let row: Option<(i64, String, String)> = conn
            .query_row(
                "SELECT id, username, password_hash FROM users WHERE username = ?1",
                params![username],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        match row {
            Some((id, username, hash)) if verify_password(password, &hash) => {
                Ok(Some(User { id, username }))
            }
            _ => Ok(None),
        }
    }

    fn create_session(&self, user_id: i64) -> Result<String, StoreError> {
        let token = random_token();
        let now = Utc::now();
        let expires = now + Duration::hours(SESSION_TTL_HOURS);
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        conn.execute(
            "INSERT INTO sessions (token, user_id, created_at_unix_ms, expires_at_unix_ms) VALUES (?1,?2,?3,?4)",
            params![token, user_id, now.timestamp_millis(), expires.timestamp_millis()],
        )?;
        Ok(token)
    }

    fn validate_session(&self, token: &str) -> Result<Option<User>, StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        let row: Option<(i64, String, i64)> = conn
            .query_row(
                "SELECT u.id, u.username, s.expires_at_unix_ms FROM sessions s
                 JOIN users u ON u.id = s.user_id WHERE s.token = ?1",
                params![token],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .ok();

        Ok(row.and_then(|(id, username, expires_ms)| {
            let expires: DateTime<Utc> = Utc.timestamp_millis_opt(expires_ms).single()?;
            if expires > Utc::now() {
                Some(User { id, username })
            } else {
                None
            }
        }))
    }

    fn delete_session(&self, token: &str) -> Result<(), StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_password_does_not_authenticate() {
        let store = SqliteAuthStore::open(":memory:").expect("open in-memory store");
        store.create_user("alice", "correct-horse").unwrap();

        assert!(store.verify_login("alice", "wrong-password").unwrap().is_none());
        assert!(store.verify_login("alice", "correct-horse").unwrap().is_some());
    }

    #[test]
    fn session_lifecycle_create_validate_delete() {
        let store = SqliteAuthStore::open(":memory:").expect("open in-memory store");
        let user = store.create_user("bob", "hunter2").unwrap();

        let token = store.create_session(user.id).unwrap();
        let validated = store.validate_session(&token).unwrap();
        assert_eq!(validated.map(|u| u.username), Some("bob".to_string()));

        store.delete_session(&token).unwrap();
        assert!(store.validate_session(&token).unwrap().is_none());
    }

    #[test]
    fn unknown_token_does_not_validate() {
        let store = SqliteAuthStore::open(":memory:").expect("open in-memory store");
        assert!(store.validate_session("nonexistent").unwrap().is_none());
    }
}

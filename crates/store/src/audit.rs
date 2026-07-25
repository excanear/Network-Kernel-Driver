use std::sync::Mutex;

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::sqlite::StoreError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub actor: String,
    pub detail: String,
}

pub trait AuditStore: Send + Sync {
    fn record(&self, event_type: &str, actor: &str, detail: &str) -> Result<(), StoreError>;
    fn list_recent(&self, limit: u32) -> Result<Vec<AuditEvent>, StoreError>;
}

pub struct SqliteAuditStore {
    conn: Mutex<Connection>,
}

impl SqliteAuditStore {
    pub fn open(path: &str) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp_unix_ms INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                actor TEXT NOT NULL,
                detail TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_log_ts ON audit_log(timestamp_unix_ms);",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }
}

impl AuditStore for SqliteAuditStore {
    fn record(&self, event_type: &str, actor: &str, detail: &str) -> Result<(), StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        conn.execute(
            "INSERT INTO audit_log (timestamp_unix_ms, event_type, actor, detail) VALUES (?1,?2,?3,?4)",
            params![Utc::now().timestamp_millis(), event_type, actor, detail],
        )?;
        Ok(())
    }

    fn list_recent(&self, limit: u32) -> Result<Vec<AuditEvent>, StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, timestamp_unix_ms, event_type, actor, detail FROM audit_log
             ORDER BY timestamp_unix_ms DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| {
            let ts_ms: i64 = row.get(1)?;
            Ok(AuditEvent {
                id: row.get(0)?,
                timestamp: Utc.timestamp_millis_opt(ts_ms).single().unwrap_or_else(Utc::now),
                event_type: row.get(2)?,
                actor: row.get(3)?,
                detail: row.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

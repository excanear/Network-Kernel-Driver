use alerts::{Alert, AlertKind, AlertSeverity};
use chrono::{TimeZone, Utc};
use rusqlite::{params, Connection};
use std::sync::Mutex;

use crate::sqlite::StoreError;

pub trait AlertStore: Send + Sync {
    fn insert_alert(&self, alert: &Alert) -> Result<(), StoreError>;
    fn resolve_alert(&self, id: &str, resolved_at: chrono::DateTime<Utc>) -> Result<(), StoreError>;
    fn list_active(&self) -> Result<Vec<Alert>, StoreError>;
    fn list_recent(&self, limit: u32) -> Result<Vec<Alert>, StoreError>;
}

pub struct SqliteAlertStore {
    conn: Mutex<Connection>,
}

fn kind_str(k: AlertKind) -> &'static str {
    k.label()
}

fn kind_from_str(s: &str) -> AlertKind {
    match s {
        "packet_loss_high" => AlertKind::PacketLossHigh,
        "link_down" => AlertKind::LinkDown,
        "interface_offline" => AlertKind::InterfaceOffline,
        "ip_address_changed" => AlertKind::IpAddressChanged,
        "speed_degraded" => AlertKind::SpeedDegraded,
        _ => AlertKind::ErrorRateHigh,
    }
}

fn severity_str(s: AlertSeverity) -> &'static str {
    match s {
        AlertSeverity::Info => "info",
        AlertSeverity::Warning => "warning",
        AlertSeverity::Critical => "critical",
    }
}

fn severity_from_str(s: &str) -> AlertSeverity {
    match s {
        "warning" => AlertSeverity::Warning,
        "critical" => AlertSeverity::Critical,
        _ => AlertSeverity::Info,
    }
}

impl SqliteAlertStore {
    pub fn open(path: &str) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS alerts (
                id TEXT PRIMARY KEY,
                if_index INTEGER NOT NULL,
                if_name TEXT NOT NULL,
                kind TEXT NOT NULL,
                severity TEXT NOT NULL,
                message TEXT NOT NULL,
                triggered_at_unix_ms INTEGER NOT NULL,
                resolved_at_unix_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_alerts_active ON alerts(resolved_at_unix_ms);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn row_to_alert(row: &rusqlite::Row) -> rusqlite::Result<Alert> {
        let triggered_ms: i64 = row.get(6)?;
        let resolved_ms: Option<i64> = row.get(7)?;
        Ok(Alert {
            id: row.get(0)?,
            if_index: row.get(1)?,
            if_name: row.get(2)?,
            kind: kind_from_str(&row.get::<_, String>(3)?),
            severity: severity_from_str(&row.get::<_, String>(4)?),
            message: row.get(5)?,
            triggered_at: Utc.timestamp_millis_opt(triggered_ms).single().unwrap_or_else(Utc::now),
            resolved_at: resolved_ms.and_then(|ms| Utc.timestamp_millis_opt(ms).single()),
        })
    }
}

impl AlertStore for SqliteAlertStore {
    fn insert_alert(&self, alert: &Alert) -> Result<(), StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO alerts (id, if_index, if_name, kind, severity, message, triggered_at_unix_ms, resolved_at_unix_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                alert.id,
                alert.if_index,
                alert.if_name,
                kind_str(alert.kind),
                severity_str(alert.severity),
                alert.message,
                alert.triggered_at.timestamp_millis(),
                alert.resolved_at.map(|d| d.timestamp_millis()),
            ],
        )?;
        Ok(())
    }

    fn resolve_alert(&self, id: &str, resolved_at: chrono::DateTime<Utc>) -> Result<(), StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        conn.execute(
            "UPDATE alerts SET resolved_at_unix_ms = ?1 WHERE id = ?2",
            params![resolved_at.timestamp_millis(), id],
        )?;
        Ok(())
    }

    fn list_active(&self) -> Result<Vec<Alert>, StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, if_index, if_name, kind, severity, message, triggered_at_unix_ms, resolved_at_unix_ms
             FROM alerts WHERE resolved_at_unix_ms IS NULL ORDER BY triggered_at_unix_ms DESC",
        )?;
        let rows = stmt.query_map([], Self::row_to_alert)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    fn list_recent(&self, limit: u32) -> Result<Vec<Alert>, StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, if_index, if_name, kind, severity, message, triggered_at_unix_ms, resolved_at_unix_ms
             FROM alerts ORDER BY triggered_at_unix_ms DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], Self::row_to_alert)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

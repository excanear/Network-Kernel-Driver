use chrono::{DateTime, TimeZone, Utc};
use collector_core::{CollectorSource, Duplex, InterfaceStats, OperStatus};
use rusqlite::{params, Connection};
use std::sync::Mutex;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("postgres error: {0}")]
    Postgres(String),
}

/// Persistence contract for historical samples, implemented today by
/// `SqliteHistoryStore`. A future Postgres/TimescaleDB-backed implementation
/// (Phase 3, see docs/roadmap.md) can satisfy this same trait without
/// changing `service`/`cli`/`web`.
pub trait HistoryStore: Send + Sync {
    fn insert_sample(&self, sample: &InterfaceStats) -> Result<(), StoreError>;
    fn query_range(
        &self,
        if_index: u32,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<InterfaceStats>, StoreError>;
}

pub struct SqliteHistoryStore {
    conn: Mutex<Connection>,
}

impl SqliteHistoryStore {
    pub fn open(path: &str) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS interface_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                if_index INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                mac_address TEXT NOT NULL,
                mtu INTEGER NOT NULL,
                oper_status TEXT NOT NULL,
                link_speed_bps INTEGER,
                duplex TEXT,
                if_type TEXT NOT NULL,
                ipv4_addresses TEXT NOT NULL,
                ipv6_addresses TEXT NOT NULL,
                rx_bytes INTEGER NOT NULL,
                tx_bytes INTEGER NOT NULL,
                rx_packets INTEGER NOT NULL,
                tx_packets INTEGER NOT NULL,
                rx_errors INTEGER NOT NULL,
                tx_errors INTEGER NOT NULL,
                rx_drops INTEGER NOT NULL,
                tx_drops INTEGER NOT NULL,
                rx_broadcast_packets INTEGER,
                rx_multicast_packets INTEGER,
                timestamp_unix_ms INTEGER NOT NULL,
                collector_source TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_interface_stats_if_index_ts
                ON interface_stats(if_index, timestamp_unix_ms);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn in_memory() -> Result<Self, StoreError> {
        Self::open(":memory:")
    }
}

fn oper_status_str(s: OperStatus) -> &'static str {
    match s {
        OperStatus::Up => "up",
        OperStatus::Down => "down",
        OperStatus::Testing => "testing",
        OperStatus::Unknown => "unknown",
        OperStatus::Dormant => "dormant",
        OperStatus::NotPresent => "not_present",
        OperStatus::LowerLayerDown => "lower_layer_down",
    }
}

fn oper_status_from_str(s: &str) -> OperStatus {
    match s {
        "up" => OperStatus::Up,
        "down" => OperStatus::Down,
        "testing" => OperStatus::Testing,
        "dormant" => OperStatus::Dormant,
        "not_present" => OperStatus::NotPresent,
        "lower_layer_down" => OperStatus::LowerLayerDown,
        _ => OperStatus::Unknown,
    }
}

fn duplex_str(d: Option<Duplex>) -> Option<&'static str> {
    match d {
        Some(Duplex::Full) => Some("full"),
        Some(Duplex::Half) => Some("half"),
        None => None,
    }
}

fn collector_source_str(s: CollectorSource) -> &'static str {
    match s {
        CollectorSource::WindowsIpHelper => "windows_ip_helper",
        CollectorSource::LinuxProcSys => "linux_proc_sys",
        CollectorSource::KernelDriver => "kernel_driver",
    }
}

fn collector_source_from_str(s: &str) -> CollectorSource {
    match s {
        "linux_proc_sys" => CollectorSource::LinuxProcSys,
        "kernel_driver" => CollectorSource::KernelDriver,
        _ => CollectorSource::WindowsIpHelper,
    }
}

impl HistoryStore for SqliteHistoryStore {
    fn insert_sample(&self, sample: &InterfaceStats) -> Result<(), StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        conn.execute(
            "INSERT INTO interface_stats (
                if_index, name, description, mac_address, mtu, oper_status,
                link_speed_bps, duplex, if_type, ipv4_addresses, ipv6_addresses,
                rx_bytes, tx_bytes, rx_packets, tx_packets, rx_errors, tx_errors,
                rx_drops, tx_drops, rx_broadcast_packets, rx_multicast_packets,
                timestamp_unix_ms, collector_source
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23)",
            params![
                sample.index,
                sample.name,
                sample.description,
                sample.mac_address,
                sample.mtu,
                oper_status_str(sample.oper_status),
                sample.link_speed_bps,
                duplex_str(sample.duplex),
                sample.if_type,
                serde_json::to_string(&sample.ipv4_addresses).unwrap_or_default(),
                serde_json::to_string(&sample.ipv6_addresses).unwrap_or_default(),
                sample.rx_bytes,
                sample.tx_bytes,
                sample.rx_packets,
                sample.tx_packets,
                sample.rx_errors,
                sample.tx_errors,
                sample.rx_drops,
                sample.tx_drops,
                sample.rx_broadcast_packets,
                sample.rx_multicast_packets,
                sample.timestamp.timestamp_millis(),
                collector_source_str(sample.collector_source),
            ],
        )?;
        Ok(())
    }

    fn query_range(
        &self,
        if_index: u32,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<InterfaceStats>, StoreError> {
        let conn = self.conn.lock().expect("sqlite connection lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT if_index, name, description, mac_address, mtu, oper_status,
                    link_speed_bps, duplex, if_type, ipv4_addresses, ipv6_addresses,
                    rx_bytes, tx_bytes, rx_packets, tx_packets, rx_errors, tx_errors,
                    rx_drops, tx_drops, rx_broadcast_packets, rx_multicast_packets,
                    timestamp_unix_ms, collector_source
             FROM interface_stats
             WHERE if_index = ?1 AND timestamp_unix_ms BETWEEN ?2 AND ?3
             ORDER BY timestamp_unix_ms DESC
             LIMIT ?4",
        )?;

        let rows = stmt.query_map(
            params![if_index, from.timestamp_millis(), to.timestamp_millis(), limit],
            |row| {
                let ipv4_json: String = row.get(9)?;
                let ipv6_json: String = row.get(10)?;
                let ts_ms: i64 = row.get(21)?;
                Ok(InterfaceStats {
                    index: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    mac_address: row.get(3)?,
                    mtu: row.get(4)?,
                    oper_status: oper_status_from_str(&row.get::<_, String>(5)?),
                    link_speed_bps: row.get(6)?,
                    duplex: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| match s.as_str() {
                            "full" => Some(Duplex::Full),
                            "half" => Some(Duplex::Half),
                            _ => None,
                        }),
                    if_type: row.get(8)?,
                    ipv4_addresses: serde_json::from_str(&ipv4_json).unwrap_or_default(),
                    ipv6_addresses: serde_json::from_str(&ipv6_json).unwrap_or_default(),
                    rx_bytes: row.get(11)?,
                    tx_bytes: row.get(12)?,
                    rx_packets: row.get(13)?,
                    tx_packets: row.get(14)?,
                    rx_errors: row.get(15)?,
                    tx_errors: row.get(16)?,
                    rx_drops: row.get(17)?,
                    tx_drops: row.get(18)?,
                    rx_broadcast_packets: row.get(19)?,
                    rx_multicast_packets: row.get(20)?,
                    timestamp: Utc.timestamp_millis_opt(ts_ms).single().unwrap_or_else(Utc::now),
                    collector_source: collector_source_from_str(&row.get::<_, String>(22)?),
                })
            },
        )?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collector_core::CollectorSource;
    use chrono::Duration;

    fn sample(index: u32, rx_bytes: u64, ts: DateTime<Utc>) -> InterfaceStats {
        InterfaceStats {
            index,
            name: "eth0".into(),
            description: String::new(),
            mac_address: "AA:BB:CC:DD:EE:FF".into(),
            mtu: 1500,
            oper_status: OperStatus::Up,
            link_speed_bps: Some(1_000_000_000),
            duplex: Some(Duplex::Full),
            if_type: "Ethernet".into(),
            ipv4_addresses: vec!["10.0.0.2".into()],
            ipv6_addresses: vec![],
            rx_bytes,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
            rx_errors: 0,
            tx_errors: 0,
            rx_drops: 0,
            tx_drops: 0,
            rx_broadcast_packets: None,
            rx_multicast_packets: None,
            timestamp: ts,
            collector_source: CollectorSource::WindowsIpHelper,
        }
    }

    #[test]
    fn insert_and_query_range_roundtrips() {
        let store = SqliteHistoryStore::in_memory().expect("open in-memory store");
        let now = Utc::now();

        store.insert_sample(&sample(1, 100, now - Duration::seconds(20))).unwrap();
        store.insert_sample(&sample(1, 200, now - Duration::seconds(10))).unwrap();
        store.insert_sample(&sample(2, 999, now)).unwrap(); // different interface

        let results = store
            .query_range(1, now - Duration::minutes(1), now + Duration::minutes(1), 10)
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|s| s.index == 1));
        // Results are ordered most-recent-first.
        assert_eq!(results[0].rx_bytes, 200);
        assert_eq!(results[1].rx_bytes, 100);
    }

    #[test]
    fn query_range_respects_limit() {
        let store = SqliteHistoryStore::in_memory().expect("open in-memory store");
        let now = Utc::now();
        for i in 0..5 {
            store.insert_sample(&sample(1, i, now - Duration::seconds(i as i64))).unwrap();
        }

        let results = store
            .query_range(1, now - Duration::minutes(1), now + Duration::minutes(1), 2)
            .unwrap();
        assert_eq!(results.len(), 2);
    }
}

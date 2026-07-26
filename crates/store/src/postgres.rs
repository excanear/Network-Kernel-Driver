use std::sync::mpsc as std_mpsc;
use std::thread;

use chrono::{DateTime, TimeZone, Utc};
use collector_core::{CollectorSource, Duplex, InterfaceStats, OperStatus};
use postgres::{Client, NoTls};

use crate::sqlite::{HistoryStore, StoreError};

/// PostgreSQL/TimescaleDB-backed implementation of `HistoryStore` (Phase F,
/// see docs/roadmap.md). Same trait, same schema shape as
/// `SqliteHistoryStore` — `service`/`cli`/`web` don't need to change to use
/// this backend; only the connection string does (`NETOBS_DATABASE_URL`).
///
/// The `postgres` crate is fully synchronous — internally it builds its own
/// single-threaded Tokio runtime to drive `tokio-postgres`, which panics
/// ("Cannot start a runtime from within a runtime") if used directly from
/// code already running inside `network-observatoryd`'s own Tokio runtime
/// (poller, REST handlers). So the client lives on a dedicated plain OS
/// thread with no Tokio context at all, and `HistoryStore` calls talk to it
/// over a request/response channel — the standard bridge pattern for a sync
/// driver inside an async service.
pub struct PostgresHistoryStore {
    tx: std_mpsc::Sender<Command>,
}

enum Command {
    Insert {
        sample: Box<InterfaceStats>,
        reply: std_mpsc::Sender<Result<(), StoreError>>,
    },
    Query {
        if_index: u32,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u32,
        reply: std_mpsc::Sender<Result<Vec<InterfaceStats>, StoreError>>,
    },
}

impl PostgresHistoryStore {
    pub fn open(connection_string: &str) -> Result<Self, StoreError> {
        let (tx, rx) = std_mpsc::channel::<Command>();
        let (ready_tx, ready_rx) = std_mpsc::channel::<Result<(), String>>();

        let connection_string = connection_string.to_string();
        thread::spawn(move || {
            let mut client = match Client::connect(&connection_string, NoTls) {
                Ok(c) => c,
                Err(e) => {
                    let _ = ready_tx.send(Err(format!("postgres connect failed: {e}")));
                    return;
                }
            };
            if let Err(e) = init_schema(&mut client) {
                let _ = ready_tx.send(Err(e));
                return;
            }
            let _ = ready_tx.send(Ok(()));

            // The connection can drop (WSL2 NAT idling, transient network
            // blips — see docs/roadmap.md Phase F). Reconnect once before
            // giving up on a command rather than poisoning every call after
            // the first hiccup.
            let reconnect = |client: &mut Client| -> bool {
                match Client::connect(&connection_string, NoTls) {
                    Ok(fresh) => {
                        *client = fresh;
                        true
                    }
                    Err(_) => false,
                }
            };

            for cmd in rx {
                match cmd {
                    Command::Insert { sample, reply } => {
                        let mut result = insert_sample_blocking(&mut client, &sample);
                        if result.is_err() && reconnect(&mut client) {
                            result = insert_sample_blocking(&mut client, &sample);
                        }
                        let _ = reply.send(result);
                    }
                    Command::Query {
                        if_index,
                        from,
                        to,
                        limit,
                        reply,
                    } => {
                        let mut result = query_range_blocking(&mut client, if_index, from, to, limit);
                        if result.is_err() && reconnect(&mut client) {
                            result = query_range_blocking(&mut client, if_index, from, to, limit);
                        }
                        let _ = reply.send(result);
                    }
                }
            }
        });

        ready_rx
            .recv()
            .map_err(|e| StoreError::Postgres(format!("postgres worker thread died: {e}")))?
            .map_err(StoreError::Postgres)?;

        Ok(Self { tx })
    }
}

fn init_schema(client: &mut Client) -> Result<(), String> {
    client
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS interface_stats (
                id BIGSERIAL PRIMARY KEY,
                if_index INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                mac_address TEXT NOT NULL,
                mtu INTEGER NOT NULL,
                oper_status TEXT NOT NULL,
                link_speed_bps BIGINT,
                duplex TEXT,
                if_type TEXT NOT NULL,
                ipv4_addresses TEXT NOT NULL,
                ipv6_addresses TEXT NOT NULL,
                rx_bytes BIGINT NOT NULL,
                tx_bytes BIGINT NOT NULL,
                rx_packets BIGINT NOT NULL,
                tx_packets BIGINT NOT NULL,
                rx_errors BIGINT NOT NULL,
                tx_errors BIGINT NOT NULL,
                rx_drops BIGINT NOT NULL,
                tx_drops BIGINT NOT NULL,
                rx_broadcast_packets BIGINT,
                rx_multicast_packets BIGINT,
                timestamp_unix_ms BIGINT NOT NULL,
                collector_source TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_interface_stats_if_index_ts
                ON interface_stats(if_index, timestamp_unix_ms);",
        )
        .map_err(|e| format!("postgres schema init failed: {e}"))?;

    // Best-effort: only succeeds if the TimescaleDB extension is installed
    // (see scripts/wsl-setup-timescaledb.sh). A plain PostgreSQL instance
    // still works fine as a regular table — the hypertable conversion is
    // purely a scale/retention optimization, not something callers depend
    // on for correctness.
    let _ = client.batch_execute("CREATE EXTENSION IF NOT EXISTS timescaledb;");
    let _ = client.batch_execute(
        "SELECT create_hypertable('interface_stats', by_range('timestamp_unix_ms'), if_not_exists => TRUE);",
    );

    Ok(())
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

fn insert_sample_blocking(client: &mut Client, sample: &InterfaceStats) -> Result<(), StoreError> {
    client
        .execute(
            "INSERT INTO interface_stats (
                if_index, name, description, mac_address, mtu, oper_status,
                link_speed_bps, duplex, if_type, ipv4_addresses, ipv6_addresses,
                rx_bytes, tx_bytes, rx_packets, tx_packets, rx_errors, tx_errors,
                rx_drops, tx_drops, rx_broadcast_packets, rx_multicast_packets,
                timestamp_unix_ms, collector_source
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23)",
            &[
                &(sample.index as i32),
                &sample.name,
                &sample.description,
                &sample.mac_address,
                &(sample.mtu as i32),
                &oper_status_str(sample.oper_status),
                &sample.link_speed_bps.map(|v| v as i64),
                &duplex_str(sample.duplex),
                &sample.if_type,
                &serde_json::to_string(&sample.ipv4_addresses).unwrap_or_default(),
                &serde_json::to_string(&sample.ipv6_addresses).unwrap_or_default(),
                &(sample.rx_bytes as i64),
                &(sample.tx_bytes as i64),
                &(sample.rx_packets as i64),
                &(sample.tx_packets as i64),
                &(sample.rx_errors as i64),
                &(sample.tx_errors as i64),
                &(sample.rx_drops as i64),
                &(sample.tx_drops as i64),
                &sample.rx_broadcast_packets.map(|v| v as i64),
                &sample.rx_multicast_packets.map(|v| v as i64),
                &sample.timestamp.timestamp_millis(),
                &collector_source_str(sample.collector_source),
            ],
        )
        .map_err(|e| StoreError::Postgres(format!("postgres insert failed: {e}")))?;
    Ok(())
}

fn query_range_blocking(
    client: &mut Client,
    if_index: u32,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    limit: u32,
) -> Result<Vec<InterfaceStats>, StoreError> {
    let rows = client
        .query(
            "SELECT if_index, name, description, mac_address, mtu, oper_status,
                    link_speed_bps, duplex, if_type, ipv4_addresses, ipv6_addresses,
                    rx_bytes, tx_bytes, rx_packets, tx_packets, rx_errors, tx_errors,
                    rx_drops, tx_drops, rx_broadcast_packets, rx_multicast_packets,
                    timestamp_unix_ms, collector_source
             FROM interface_stats
             WHERE if_index = $1 AND timestamp_unix_ms BETWEEN $2 AND $3
             ORDER BY timestamp_unix_ms DESC
             LIMIT $4",
            &[
                &(if_index as i32),
                &from.timestamp_millis(),
                &to.timestamp_millis(),
                &(limit as i64),
            ],
        )
        .map_err(|e| StoreError::Postgres(format!("postgres query failed: {e}")))?;

    let mut results = Vec::with_capacity(rows.len());
    for row in rows {
        let ipv4_json: String = row.get(9);
        let ipv6_json: String = row.get(10);
        let ts_ms: i64 = row.get(21);
        results.push(InterfaceStats {
            index: row.get::<_, i32>(0) as u32,
            name: row.get(1),
            description: row.get(2),
            mac_address: row.get(3),
            mtu: row.get::<_, i32>(4) as u32,
            oper_status: oper_status_from_str(row.get(5)),
            link_speed_bps: row.get::<_, Option<i64>>(6).map(|v| v as u64),
            duplex: row.get::<_, Option<String>>(7).and_then(|s| match s.as_str() {
                "full" => Some(Duplex::Full),
                "half" => Some(Duplex::Half),
                _ => None,
            }),
            if_type: row.get(8),
            ipv4_addresses: serde_json::from_str(&ipv4_json).unwrap_or_default(),
            ipv6_addresses: serde_json::from_str(&ipv6_json).unwrap_or_default(),
            rx_bytes: row.get::<_, i64>(11) as u64,
            tx_bytes: row.get::<_, i64>(12) as u64,
            rx_packets: row.get::<_, i64>(13) as u64,
            tx_packets: row.get::<_, i64>(14) as u64,
            rx_errors: row.get::<_, i64>(15) as u64,
            tx_errors: row.get::<_, i64>(16) as u64,
            rx_drops: row.get::<_, i64>(17) as u64,
            tx_drops: row.get::<_, i64>(18) as u64,
            rx_broadcast_packets: row.get::<_, Option<i64>>(19).map(|v| v as u64),
            rx_multicast_packets: row.get::<_, Option<i64>>(20).map(|v| v as u64),
            timestamp: Utc.timestamp_millis_opt(ts_ms).single().unwrap_or_else(Utc::now),
            collector_source: collector_source_from_str(row.get(22)),
        });
    }
    Ok(results)
}

impl HistoryStore for PostgresHistoryStore {
    fn insert_sample(&self, sample: &InterfaceStats) -> Result<(), StoreError> {
        let (reply_tx, reply_rx) = std_mpsc::channel();
        self.tx
            .send(Command::Insert {
                sample: Box::new(sample.clone()),
                reply: reply_tx,
            })
            .map_err(|_| StoreError::Postgres("postgres worker thread is gone".into()))?;
        reply_rx
            .recv()
            .map_err(|_| StoreError::Postgres("postgres worker thread dropped reply".into()))?
    }

    fn query_range(
        &self,
        if_index: u32,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<InterfaceStats>, StoreError> {
        let (reply_tx, reply_rx) = std_mpsc::channel();
        self.tx
            .send(Command::Query {
                if_index,
                from,
                to,
                limit,
                reply: reply_tx,
            })
            .map_err(|_| StoreError::Postgres("postgres worker thread is gone".into()))?;
        reply_rx
            .recv()
            .map_err(|_| StoreError::Postgres("postgres worker thread dropped reply".into()))?
    }
}

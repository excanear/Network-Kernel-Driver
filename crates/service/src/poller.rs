use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use alerts::AlertEngine;
use collector_core::InterfaceCollector;
use store::{AlertStore, AuditStore, HistoryStore, RingBufferStore};
use tokio::sync::{broadcast, Mutex};
use tokio::time::interval;
use tracing::{error, info, warn};

use collector_core::Snapshot;

pub struct Poller {
    pub collector: Arc<dyn InterfaceCollector>,
    pub ring_buffer: Arc<RingBufferStore>,
    pub history: Arc<dyn HistoryStore>,
    pub alert_store: Arc<dyn AlertStore>,
    pub audit_store: Arc<dyn AuditStore>,
    pub alert_engine: Arc<Mutex<AlertEngine>>,
    pub tx: broadcast::Sender<Snapshot>,
    pub poll_interval: Duration,
    pub persist_every_n_ticks: u32,
}

impl Poller {
    pub async fn run(self) {
        let mut ticker = interval(self.poll_interval);
        let mut tick_count: u32 = 0;

        loop {
            ticker.tick().await;
            tick_count += 1;

            // Snapshot of "previous" state, taken before this tick's push,
            // so the alert engine can diff consecutive samples per interface.
            let prev_by_index: HashMap<u32, collector_core::InterfaceStats> = self
                .ring_buffer
                .latest_all()
                .into_iter()
                .map(|s| (s.index, s))
                .collect();

            match self.collector.snapshot() {
                Ok(snapshot) => {
                    self.ring_buffer.push_all(&snapshot.interfaces);

                    if tick_count % self.persist_every_n_ticks == 0 {
                        for sample in &snapshot.interfaces {
                            if let Err(e) = self.history.insert_sample(sample) {
                                error!("failed to persist sample for if {}: {e}", sample.index);
                            }
                        }
                    }

                    {
                        let mut engine = self.alert_engine.lock().await;
                        for sample in &snapshot.interfaces {
                            let prev = prev_by_index.get(&sample.index);
                            let result = engine.evaluate(prev, sample);
                            for alert in &result.triggered {
                                if let Err(e) = self.alert_store.insert_alert(alert) {
                                    warn!("failed to persist alert {}: {e}", alert.id);
                                }
                                let _ = self.audit_store.record(
                                    "alert.triggered",
                                    "system",
                                    &format!("{:?} {} — {}", alert.severity, alert.if_name, alert.message),
                                );
                            }
                            for alert in &result.resolved {
                                if let Err(e) = self.alert_store.insert_alert(alert) {
                                    warn!("failed to persist alert {}: {e}", alert.id);
                                }
                                let _ = self.audit_store.record(
                                    "alert.resolved",
                                    "system",
                                    &format!("{} — {}", alert.if_name, alert.message),
                                );
                            }
                        }
                    }

                    // Ignore send errors: they only mean no WS subscribers are connected.
                    let _ = self.tx.send(snapshot);
                }
                Err(e) => {
                    error!("collector snapshot failed: {e}");
                }
            }
        }
    }

    pub fn spawn(self) {
        info!(
            "starting poller: backend={}, interval={:?}",
            self.collector.platform_name(),
            self.poll_interval
        );
        tokio::spawn(self.run());
    }
}

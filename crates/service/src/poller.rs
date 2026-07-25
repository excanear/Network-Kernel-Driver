use std::sync::Arc;
use std::time::Duration;

use collector_core::InterfaceCollector;
use store::{HistoryStore, RingBufferStore};
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{error, info};

use collector_core::Snapshot;

pub struct Poller {
    pub collector: Arc<dyn InterfaceCollector>,
    pub ring_buffer: Arc<RingBufferStore>,
    pub history: Arc<dyn HistoryStore>,
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

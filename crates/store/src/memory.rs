use std::collections::{HashMap, VecDeque};
use std::sync::RwLock;

use collector_core::InterfaceStats;

const DEFAULT_CAPACITY: usize = 300;

/// Bounded per-interface ring buffer, kept in memory to feed the live
/// dashboard chart without touching disk on every poll tick.
pub struct RingBufferStore {
    capacity: usize,
    buffers: RwLock<HashMap<u32, VecDeque<InterfaceStats>>>,
}

impl RingBufferStore {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            buffers: RwLock::new(HashMap::new()),
        }
    }

    pub fn push_all(&self, interfaces: &[InterfaceStats]) {
        let mut buffers = self.buffers.write().expect("ring buffer lock poisoned");
        for stat in interfaces {
            let entry = buffers.entry(stat.index).or_insert_with(VecDeque::new);
            entry.push_back(stat.clone());
            while entry.len() > self.capacity {
                entry.pop_front();
            }
        }
    }

    pub fn recent(&self, if_index: u32) -> Vec<InterfaceStats> {
        let buffers = self.buffers.read().expect("ring buffer lock poisoned");
        buffers
            .get(&if_index)
            .map(|d| d.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Latest sample for every tracked interface, i.e. the most recent poll
    /// tick's snapshot — avoids callers hitting the platform collector
    /// directly for reads that the poller has already gathered.
    pub fn latest_all(&self) -> Vec<InterfaceStats> {
        let buffers = self.buffers.read().expect("ring buffer lock poisoned");
        buffers.values().filter_map(|d| d.back().cloned()).collect()
    }
}

impl Default for RingBufferStore {
    fn default() -> Self {
        Self::new()
    }
}

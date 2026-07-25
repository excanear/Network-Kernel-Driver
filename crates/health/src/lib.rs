use chrono::{DateTime, Utc};
use collector_core::{InterfaceStats, OperStatus};
use serde::{Deserialize, Serialize};

/// Health score for one interface, computed from a chronologically-ordered
/// window of samples (oldest first — matches `RingBufferStore::recent`).
///
/// Weights are a pragmatic default for Phase C: availability and stability
/// dominate because they're the strongest signal we can compute from data
/// already collected (no active probing yet — see docs/roadmap.md for
/// latency/jitter/RTT, which depend on active measurement, not just counters).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthScore {
    pub if_index: u32,
    pub score: f64,
    pub availability_pct: f64,
    pub stability_score: f64,
    pub packet_loss_pct: f64,
    pub error_rate_pct: f64,
    pub link_state_changes: u32,
    pub sample_count: usize,
    pub computed_at: DateTime<Utc>,
}

const WEIGHT_AVAILABILITY: f64 = 0.40;
const WEIGHT_STABILITY: f64 = 0.25;
const WEIGHT_PACKET_LOSS: f64 = 0.20;
const WEIGHT_ERROR_RATE: f64 = 0.15;

pub fn compute_health(if_index: u32, history: &[InterfaceStats]) -> HealthScore {
    if history.is_empty() {
        return HealthScore {
            if_index,
            score: 0.0,
            availability_pct: 0.0,
            stability_score: 0.0,
            packet_loss_pct: 0.0,
            error_rate_pct: 0.0,
            link_state_changes: 0,
            sample_count: 0,
            computed_at: Utc::now(),
        };
    }

    let up_count = history.iter().filter(|s| s.oper_status == OperStatus::Up).count();
    let availability_pct = 100.0 * up_count as f64 / history.len() as f64;

    let mut link_state_changes: u32 = 0;
    let mut rx_packets_delta_sum: i64 = 0;
    let mut rx_drops_delta_sum: i64 = 0;
    let mut errors_delta_sum: i64 = 0;
    let mut packets_delta_sum: i64 = 0;

    for pair in history.windows(2) {
        let (prev, cur) = (&pair[0], &pair[1]);
        if prev.oper_status != cur.oper_status {
            link_state_changes += 1;
        }

        let rx_packets_delta = cur.rx_packets as i64 - prev.rx_packets as i64;
        let rx_drops_delta = cur.rx_drops as i64 - prev.rx_drops as i64;
        let errors_delta = (cur.rx_errors as i64 + cur.tx_errors as i64)
            - (prev.rx_errors as i64 + prev.tx_errors as i64);
        let packets_delta = (cur.rx_packets as i64 + cur.tx_packets as i64)
            - (prev.rx_packets as i64 + prev.tx_packets as i64);

        // Skip windows where counters reset (adapter reset / overflow) rather
        // than treat a negative delta as loss.
        if rx_packets_delta >= 0 && rx_drops_delta >= 0 {
            rx_packets_delta_sum += rx_packets_delta;
            rx_drops_delta_sum += rx_drops_delta;
        }
        if errors_delta >= 0 && packets_delta >= 0 {
            errors_delta_sum += errors_delta;
            packets_delta_sum += packets_delta;
        }
    }

    let stability_penalty = (link_state_changes as f64) * 8.0;
    let stability_score = (100.0 - stability_penalty).clamp(0.0, 100.0);

    let packet_loss_pct = if rx_packets_delta_sum + rx_drops_delta_sum > 0 {
        100.0 * rx_drops_delta_sum as f64 / (rx_packets_delta_sum + rx_drops_delta_sum) as f64
    } else {
        0.0
    };

    let error_rate_pct = if packets_delta_sum > 0 {
        100.0 * errors_delta_sum as f64 / packets_delta_sum as f64
    } else {
        0.0
    };

    let score = (availability_pct * WEIGHT_AVAILABILITY
        + stability_score * WEIGHT_STABILITY
        + (100.0 - packet_loss_pct.min(100.0)) * WEIGHT_PACKET_LOSS
        + (100.0 - error_rate_pct.min(100.0)) * WEIGHT_ERROR_RATE)
        .clamp(0.0, 100.0);

    HealthScore {
        if_index,
        score,
        availability_pct,
        stability_score,
        packet_loss_pct,
        error_rate_pct,
        link_state_changes,
        sample_count: history.len(),
        computed_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collector_core::{CollectorSource, Duplex};

    fn sample(rx_packets: u64, rx_drops: u64, oper_status: OperStatus) -> InterfaceStats {
        InterfaceStats {
            index: 1,
            name: "eth0".into(),
            description: String::new(),
            mac_address: String::new(),
            mtu: 1500,
            oper_status,
            link_speed_bps: Some(1_000_000_000),
            duplex: Some(Duplex::Full),
            if_type: "Ethernet".into(),
            ipv4_addresses: vec![],
            ipv6_addresses: vec![],
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets,
            tx_packets: 0,
            rx_errors: 0,
            tx_errors: 0,
            rx_drops,
            tx_drops: 0,
            rx_broadcast_packets: None,
            rx_multicast_packets: None,
            timestamp: Utc::now(),
            collector_source: CollectorSource::WindowsIpHelper,
        }
    }

    #[test]
    fn perfect_link_scores_100() {
        let history = vec![
            sample(0, 0, OperStatus::Up),
            sample(1000, 0, OperStatus::Up),
            sample(2000, 0, OperStatus::Up),
        ];
        let health = compute_health(1, &history);
        assert!((health.score - 100.0).abs() < 0.01, "score={}", health.score);
        assert_eq!(health.link_state_changes, 0);
    }

    #[test]
    fn packet_loss_lowers_score() {
        let history = vec![
            sample(0, 0, OperStatus::Up),
            sample(1000, 500, OperStatus::Up),
        ];
        let health = compute_health(1, &history);
        assert!(health.packet_loss_pct > 0.0);
        assert!(health.score < 100.0);
    }

    #[test]
    fn link_flap_lowers_stability() {
        let history = vec![
            sample(0, 0, OperStatus::Up),
            sample(0, 0, OperStatus::Down),
            sample(0, 0, OperStatus::Up),
        ];
        let health = compute_health(1, &history);
        assert_eq!(health.link_state_changes, 2);
        assert!(health.stability_score < 100.0);
    }

    #[test]
    fn empty_history_scores_zero() {
        let health = compute_health(1, &[]);
        assert_eq!(health.score, 0.0);
        assert_eq!(health.sample_count, 0);
    }
}

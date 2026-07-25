use std::collections::HashMap;

use chrono::{DateTime, Utc};
use collector_core::{InterfaceStats, OperStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Which rule fired. Scoped to Phase C to what's derivable from interface
/// counters/state alone. Gateway/DNS-offline and DNS-latency rules are
/// deferred to Phase D (docs/roadmap.md) since they need topology data
/// (default route, resolver) not yet collected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertKind {
    PacketLossHigh,
    LinkDown,
    InterfaceOffline,
    IpAddressChanged,
    SpeedDegraded,
    ErrorRateHigh,
}

impl AlertKind {
    pub fn label(self) -> &'static str {
        match self {
            AlertKind::PacketLossHigh => "packet_loss_high",
            AlertKind::LinkDown => "link_down",
            AlertKind::InterfaceOffline => "interface_offline",
            AlertKind::IpAddressChanged => "ip_address_changed",
            AlertKind::SpeedDegraded => "speed_degraded",
            AlertKind::ErrorRateHigh => "error_rate_high",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub if_index: u32,
    pub if_name: String,
    pub kind: AlertKind,
    pub severity: AlertSeverity,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct AlertRuleConfig {
    pub packet_loss_pct_threshold: f64,
    pub error_rate_pct_threshold: f64,
    /// Fraction (0.0-1.0) below the highest speed ever observed for an
    /// interface that counts as "negotiated speed reduced".
    pub speed_degradation_ratio: f64,
}

impl Default for AlertRuleConfig {
    fn default() -> Self {
        Self {
            packet_loss_pct_threshold: 1.0,
            error_rate_pct_threshold: 1.0,
            speed_degradation_ratio: 0.5,
        }
    }
}

#[derive(Default)]
pub struct EvaluationResult {
    pub triggered: Vec<Alert>,
    pub resolved: Vec<Alert>,
}

/// Stateful rule engine: tracks which (interface, rule) pairs are currently
/// active so a condition that stays true doesn't re-fire every poll tick, and
/// emits a resolution when the condition clears.
pub struct AlertEngine {
    config: AlertRuleConfig,
    active: HashMap<(u32, AlertKind), Alert>,
    max_speed_seen: HashMap<u32, u64>,
}

impl AlertEngine {
    pub fn new(config: AlertRuleConfig) -> Self {
        Self {
            config,
            active: HashMap::new(),
            max_speed_seen: HashMap::new(),
        }
    }

    pub fn active_alerts(&self) -> Vec<Alert> {
        self.active.values().cloned().collect()
    }

    pub fn evaluate(
        &mut self,
        prev: Option<&InterfaceStats>,
        cur: &InterfaceStats,
    ) -> EvaluationResult {
        let mut result = EvaluationResult::default();

        if let Some(speed) = cur.link_speed_bps {
            let entry = self.max_speed_seen.entry(cur.index).or_insert(speed);
            if speed > *entry {
                *entry = speed;
            }
        }

        let mut conditions: HashMap<AlertKind, (AlertSeverity, String)> = HashMap::new();

        match cur.oper_status {
            OperStatus::Down => {
                conditions.insert(
                    AlertKind::LinkDown,
                    (AlertSeverity::Critical, format!("Link down on {}", cur.name)),
                );
            }
            OperStatus::NotPresent => {
                conditions.insert(
                    AlertKind::InterfaceOffline,
                    (
                        AlertSeverity::Critical,
                        format!("Interface {} is offline (not present)", cur.name),
                    ),
                );
            }
            _ => {}
        }

        if let Some(prev) = prev {
            let rx_packets_delta = cur.rx_packets as i64 - prev.rx_packets as i64;
            let rx_drops_delta = cur.rx_drops as i64 - prev.rx_drops as i64;
            if rx_packets_delta >= 0 && rx_drops_delta >= 0 && rx_packets_delta + rx_drops_delta > 0
            {
                let loss_pct =
                    100.0 * rx_drops_delta as f64 / (rx_packets_delta + rx_drops_delta) as f64;
                if loss_pct > self.config.packet_loss_pct_threshold {
                    conditions.insert(
                        AlertKind::PacketLossHigh,
                        (
                            AlertSeverity::Warning,
                            format!("Packet loss {loss_pct:.2}% on {}", cur.name),
                        ),
                    );
                }
            }

            let errors_delta = (cur.rx_errors as i64 + cur.tx_errors as i64)
                - (prev.rx_errors as i64 + prev.tx_errors as i64);
            let packets_delta = (cur.rx_packets as i64 + cur.tx_packets as i64)
                - (prev.rx_packets as i64 + prev.tx_packets as i64);
            if errors_delta >= 0 && packets_delta >= 0 && packets_delta > 0 {
                let error_pct = 100.0 * errors_delta as f64 / packets_delta as f64;
                if error_pct > self.config.error_rate_pct_threshold {
                    conditions.insert(
                        AlertKind::ErrorRateHigh,
                        (
                            AlertSeverity::Warning,
                            format!("Error rate {error_pct:.2}% on {}", cur.name),
                        ),
                    );
                }
            }

            let mut prev_ips: Vec<&String> =
                prev.ipv4_addresses.iter().chain(prev.ipv6_addresses.iter()).collect();
            let mut cur_ips: Vec<&String> =
                cur.ipv4_addresses.iter().chain(cur.ipv6_addresses.iter()).collect();
            prev_ips.sort();
            cur_ips.sort();
            if prev_ips != cur_ips && !cur_ips.is_empty() {
                conditions.insert(
                    AlertKind::IpAddressChanged,
                    (
                        AlertSeverity::Info,
                        format!("IP address changed on {}", cur.name),
                    ),
                );
            }
        }

        if let (Some(speed), Some(&max_speed)) =
            (cur.link_speed_bps, self.max_speed_seen.get(&cur.index))
        {
            if max_speed > 0 && (speed as f64) < (max_speed as f64) * self.config.speed_degradation_ratio
            {
                conditions.insert(
                    AlertKind::SpeedDegraded,
                    (
                        AlertSeverity::Warning,
                        format!(
                            "Negotiated speed dropped to {:.0} Mbps (was up to {:.0} Mbps) on {}",
                            speed as f64 / 1_000_000.0,
                            max_speed as f64 / 1_000_000.0,
                            cur.name
                        ),
                    ),
                );
            }
        }

        for kind in [
            AlertKind::PacketLossHigh,
            AlertKind::LinkDown,
            AlertKind::InterfaceOffline,
            AlertKind::IpAddressChanged,
            AlertKind::SpeedDegraded,
            AlertKind::ErrorRateHigh,
        ] {
            let key = (cur.index, kind);
            match conditions.remove(&kind) {
                Some((severity, message)) => {
                    if !self.active.contains_key(&key) {
                        let alert = Alert {
                            id: format!("{}-{}-{}", cur.index, kind.label(), Utc::now().timestamp_millis()),
                            if_index: cur.index,
                            if_name: cur.name.clone(),
                            kind,
                            severity,
                            message,
                            triggered_at: Utc::now(),
                            resolved_at: None,
                        };
                        self.active.insert(key, alert.clone());
                        result.triggered.push(alert);
                    }
                }
                None => {
                    if let Some(mut alert) = self.active.remove(&key) {
                        alert.resolved_at = Some(Utc::now());
                        result.resolved.push(alert);
                    }
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use collector_core::{CollectorSource, Duplex};

    fn sample(
        oper_status: OperStatus,
        rx_packets: u64,
        rx_drops: u64,
        link_speed_bps: Option<u64>,
    ) -> InterfaceStats {
        InterfaceStats {
            index: 1,
            name: "eth0".into(),
            description: String::new(),
            mac_address: String::new(),
            mtu: 1500,
            oper_status,
            link_speed_bps,
            duplex: Some(Duplex::Full),
            if_type: "Ethernet".into(),
            ipv4_addresses: vec!["10.0.0.2".into()],
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
    fn link_down_triggers_once_then_resolves_on_recovery() {
        let mut engine = AlertEngine::new(AlertRuleConfig::default());
        let down = sample(OperStatus::Down, 0, 0, Some(1_000_000_000));
        let up = sample(OperStatus::Up, 0, 0, Some(1_000_000_000));

        let first = engine.evaluate(None, &down);
        assert_eq!(first.triggered.len(), 1);
        assert_eq!(first.triggered[0].kind, AlertKind::LinkDown);

        // Same condition on the next tick must not re-trigger.
        let second = engine.evaluate(Some(&down), &down);
        assert!(second.triggered.is_empty());

        let recovered = engine.evaluate(Some(&down), &up);
        assert_eq!(recovered.resolved.len(), 1);
        assert_eq!(recovered.resolved[0].kind, AlertKind::LinkDown);
    }

    #[test]
    fn packet_loss_above_threshold_triggers() {
        let mut engine = AlertEngine::new(AlertRuleConfig::default());
        let prev = sample(OperStatus::Up, 0, 0, Some(1_000_000_000));
        let cur = sample(OperStatus::Up, 100, 50, Some(1_000_000_000)); // 33% loss

        let result = engine.evaluate(Some(&prev), &cur);
        assert!(result.triggered.iter().any(|a| a.kind == AlertKind::PacketLossHigh));
    }

    #[test]
    fn speed_degradation_detected_against_historical_max() {
        let mut engine = AlertEngine::new(AlertRuleConfig::default());
        let fast = sample(OperStatus::Up, 0, 0, Some(1_000_000_000));
        let slow = sample(OperStatus::Up, 100, 0, Some(100_000_000)); // 10x slower

        engine.evaluate(None, &fast);
        let result = engine.evaluate(Some(&fast), &slow);
        assert!(result.triggered.iter().any(|a| a.kind == AlertKind::SpeedDegraded));
    }

    #[test]
    fn ip_change_triggers_info_alert() {
        let mut engine = AlertEngine::new(AlertRuleConfig::default());
        let mut prev = sample(OperStatus::Up, 0, 0, Some(1_000_000_000));
        prev.ipv4_addresses = vec!["10.0.0.2".into()];
        let mut cur = sample(OperStatus::Up, 0, 0, Some(1_000_000_000));
        cur.ipv4_addresses = vec!["10.0.0.99".into()];

        let result = engine.evaluate(Some(&prev), &cur);
        assert!(result.triggered.iter().any(|a| a.kind == AlertKind::IpAddressChanged));
    }
}

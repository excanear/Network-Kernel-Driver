use alerts::Alert;
use chrono::{DateTime, Utc};
use collector_core::InterfaceStats;
use health::HealthScore;
use serde::Serialize;

mod csv_format;
mod html_format;
mod json_format;
mod markdown_format;
mod pdf_format;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Csv,
    Json,
    Html,
    Markdown,
    Pdf,
}

impl ReportFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            "html" => Some(Self::Html),
            "md" | "markdown" => Some(Self::Markdown),
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            ReportFormat::Csv => "text/csv",
            ReportFormat::Json => "application/json",
            ReportFormat::Html => "text/html",
            ReportFormat::Markdown => "text/markdown",
            ReportFormat::Pdf => "application/pdf",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ReportFormat::Csv => "csv",
            ReportFormat::Json => "json",
            ReportFormat::Html => "html",
            ReportFormat::Markdown => "md",
            ReportFormat::Pdf => "pdf",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportInput {
    pub generated_at: DateTime<Utc>,
    pub interfaces: Vec<InterfaceStats>,
    pub health: Vec<HealthScore>,
    pub alerts: Vec<Alert>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("pdf generation failed: {0}")]
    Pdf(String),
}

pub fn generate(format: ReportFormat, input: &ReportInput) -> Result<Vec<u8>, ReportError> {
    match format {
        ReportFormat::Csv => Ok(csv_format::render(input)),
        ReportFormat::Json => Ok(json_format::render(input)),
        ReportFormat::Html => Ok(html_format::render(input)),
        ReportFormat::Markdown => Ok(markdown_format::render(input)),
        ReportFormat::Pdf => pdf_format::render(input),
    }
}

pub(crate) fn overall_health_summary(health: &[HealthScore]) -> (f64, Option<&HealthScore>) {
    if health.is_empty() {
        return (0.0, None);
    }
    let avg = health.iter().map(|h| h.score).sum::<f64>() / health.len() as f64;
    let worst = health.iter().min_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
    (avg, worst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use collector_core::{CollectorSource, OperStatus};

    fn sample_input() -> ReportInput {
        ReportInput {
            generated_at: Utc::now(),
            interfaces: vec![InterfaceStats {
                index: 1,
                name: "eth0".into(),
                description: "Test NIC".into(),
                mac_address: "AA:BB:CC:DD:EE:FF".into(),
                mtu: 1500,
                oper_status: OperStatus::Up,
                link_speed_bps: Some(1_000_000_000),
                duplex: None,
                if_type: "Ethernet".into(),
                ipv4_addresses: vec!["10.0.0.2".into()],
                ipv6_addresses: vec![],
                rx_bytes: 1000,
                tx_bytes: 2000,
                rx_packets: 10,
                tx_packets: 20,
                rx_errors: 0,
                tx_errors: 0,
                rx_drops: 0,
                tx_drops: 0,
                rx_broadcast_packets: None,
                rx_multicast_packets: None,
                timestamp: Utc::now(),
                collector_source: CollectorSource::WindowsIpHelper,
            }],
            health: vec![],
            alerts: vec![],
        }
    }

    #[test]
    fn all_formats_produce_non_empty_output() {
        let input = sample_input();
        for format in [
            ReportFormat::Csv,
            ReportFormat::Json,
            ReportFormat::Html,
            ReportFormat::Markdown,
            ReportFormat::Pdf,
        ] {
            let bytes = generate(format, &input).expect("report generation should succeed");
            assert!(!bytes.is_empty(), "{format:?} produced empty output");
        }
    }
}

impl std::fmt::Debug for ReportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

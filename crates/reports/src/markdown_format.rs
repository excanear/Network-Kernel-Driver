use crate::{overall_health_summary, ReportInput};

pub fn render(input: &ReportInput) -> Vec<u8> {
    let (avg_score, worst) = overall_health_summary(&input.health);
    let up_count = input.interfaces.iter().filter(|i| format!("{:?}", i.oper_status) == "Up").count();

    let mut md = String::new();
    md.push_str("# Network Observatory — Report\n\n");
    md.push_str(&format!("Generated at {}\n\n", input.generated_at));

    md.push_str("## Executive Summary\n\n");
    md.push_str(&format!("- {} interfaces observed, {} currently up\n", input.interfaces.len(), up_count));
    md.push_str(&format!("- Average health score: {avg_score:.1}/100\n"));
    if let Some(w) = worst {
        md.push_str(&format!("- Least healthy interface: {} ({:.1}/100)\n", w.if_index, w.score));
    }
    md.push_str(&format!("- {} alerts in this report window\n\n", input.alerts.len()));

    md.push_str("## Metrics\n\n");
    md.push_str("| Index | Name | Status | RX | TX | Errors | Drops |\n|---|---|---|---|---|---|---|\n");
    for iface in &input.interfaces {
        md.push_str(&format!(
            "| {} | {} | {:?} | {} | {} | {} | {} |\n",
            iface.index, iface.name, iface.oper_status, iface.rx_bytes, iface.tx_bytes,
            iface.rx_errors + iface.tx_errors, iface.rx_drops + iface.tx_drops
        ));
    }

    md.push_str("\n## Health\n\n");
    md.push_str("| Index | Score | Availability % | Loss % | Error % |\n|---|---|---|---|---|\n");
    for h in &input.health {
        md.push_str(&format!(
            "| {} | {:.1} | {:.1} | {:.2} | {:.2} |\n",
            h.if_index, h.score, h.availability_pct, h.packet_loss_pct, h.error_rate_pct
        ));
    }

    md.push_str("\n## Events / Alerts\n\n");
    if input.alerts.is_empty() {
        md.push_str("No alerts in this window.\n");
    } else {
        md.push_str("| Severity | Interface | Message | Triggered | Resolved |\n|---|---|---|---|---|\n");
        for a in &input.alerts {
            md.push_str(&format!(
                "| {:?} | {} | {} | {} | {} |\n",
                a.severity, a.if_name, a.message, a.triggered_at,
                a.resolved_at.map(|d| d.to_string()).unwrap_or_else(|| "—".to_string())
            ));
        }
    }

    md.push_str("\n## Conclusions\n\n");
    if avg_score >= 90.0 {
        md.push_str("Overall network health is good across the observed interfaces.\n");
    } else if avg_score >= 70.0 {
        md.push_str("Overall network health shows some degradation — review the alerts and least-healthy interfaces above.\n");
    } else {
        md.push_str("Overall network health is poor — multiple interfaces show instability, packet loss, or downtime.\n");
    }

    md.into_bytes()
}

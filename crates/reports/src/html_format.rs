use crate::{overall_health_summary, ReportInput};

fn health_bar_chart(input: &ReportInput) -> String {
    if input.health.is_empty() {
        return String::new();
    }
    let bar_width = 28;
    let gap = 8;
    let height = 140;
    let mut bars = String::new();
    for (i, h) in input.health.iter().enumerate() {
        let x = i as i64 * (bar_width + gap);
        let bar_h = (h.score / 100.0 * (height as f64 - 20.0)).max(2.0);
        let y = height as f64 - bar_h;
        let color = if h.score >= 90.0 {
            "#0ca30c"
        } else if h.score >= 70.0 {
            "#fab219"
        } else {
            "#d03b3b"
        };
        bars.push_str(&format!(
            "<rect x=\"{x}\" y=\"{y:.0}\" width=\"{bar_width}\" height=\"{bar_h:.0}\" fill=\"{color}\" rx=\"3\"/>\
             <text x=\"{}\" y=\"{}\" font-size=\"10\" text-anchor=\"middle\" fill=\"#898781\">{}</text>",
            x + bar_width / 2,
            height + 14,
            h.if_index
        ));
    }
    let width = input.health.len() as i64 * (bar_width + gap);
    format!(
        "<svg width=\"{width}\" height=\"{}\" xmlns=\"http://www.w3.org/2000/svg\">{bars}</svg>",
        height + 20
    )
}

pub fn render(input: &ReportInput) -> Vec<u8> {
    let (avg_score, worst) = overall_health_summary(&input.health);
    let up_count = input.interfaces.iter().filter(|i| format!("{:?}", i.oper_status) == "Up").count();

    let mut html = String::new();
    html.push_str(&format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Network Observatory Report</title>\
         <style>body{{font-family:system-ui,sans-serif;background:#0d0d0d;color:#fff;padding:32px}}\
         h1,h2{{color:#fff}} table{{border-collapse:collapse;width:100%;margin-bottom:24px}}\
         th,td{{border-bottom:1px solid #2c2c2a;padding:6px 10px;text-align:left;font-size:13px}}\
         th{{color:#898781;text-transform:uppercase;font-size:11px}}\
         .muted{{color:#898781}} .critical{{color:#d03b3b}} .warning{{color:#fab219}} .good{{color:#0ca30c}}\
         </style></head><body>"
    ));

    html.push_str(&format!("<h1>Network Observatory — Report</h1><p class=\"muted\">Generated at {}</p>", input.generated_at));

    html.push_str("<h2>Executive Summary</h2><ul>");
    html.push_str(&format!("<li>{} interfaces observed, {} currently up</li>", input.interfaces.len(), up_count));
    html.push_str(&format!("<li>Average health score: {avg_score:.1}/100</li>"));
    if let Some(w) = worst {
        html.push_str(&format!("<li>Least healthy interface: {} ({:.1}/100)</li>", w.if_index, w.score));
    }
    html.push_str(&format!("<li>{} alerts in this report window</li></ul>", input.alerts.len()));

    html.push_str("<h2>Health</h2>");
    html.push_str(&health_bar_chart(input));

    html.push_str("<h2>Metrics</h2><table><tr><th>Index</th><th>Name</th><th>Status</th><th>RX</th><th>TX</th><th>Errors</th><th>Drops</th></tr>");
    for iface in &input.interfaces {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{:?}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            iface.index, iface.name, iface.oper_status, iface.rx_bytes, iface.tx_bytes,
            iface.rx_errors + iface.tx_errors, iface.rx_drops + iface.tx_drops
        ));
    }
    html.push_str("</table>");

    html.push_str("<h2>Events / Alerts</h2>");
    if input.alerts.is_empty() {
        html.push_str("<p class=\"muted\">No alerts in this window.</p>");
    } else {
        html.push_str("<table><tr><th>Severity</th><th>Interface</th><th>Message</th><th>Triggered</th><th>Resolved</th></tr>");
        for a in &input.alerts {
            let sev_class = match a.severity {
                alerts::AlertSeverity::Critical => "critical",
                alerts::AlertSeverity::Warning => "warning",
                alerts::AlertSeverity::Info => "good",
            };
            html.push_str(&format!(
                "<tr><td class=\"{sev_class}\">{:?}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                a.severity, a.if_name, a.message, a.triggered_at,
                a.resolved_at.map(|d| d.to_string()).unwrap_or_else(|| "—".to_string())
            ));
        }
        html.push_str("</table>");
    }

    html.push_str("<h2>Conclusions</h2><p>");
    if avg_score >= 90.0 {
        html.push_str("Overall network health is good across the observed interfaces.");
    } else if avg_score >= 70.0 {
        html.push_str("Overall network health shows some degradation — review the alerts and least-healthy interfaces above.");
    } else {
        html.push_str("Overall network health is poor — multiple interfaces show instability, packet loss, or downtime.");
    }
    html.push_str("</p>");

    html.push_str("</body></html>");
    html.into_bytes()
}

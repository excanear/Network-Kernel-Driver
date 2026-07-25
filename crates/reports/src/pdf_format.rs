use printpdf::*;

use crate::{overall_health_summary, ReportError, ReportInput};

pub fn render(input: &ReportInput) -> Result<Vec<u8>, ReportError> {
    let (doc, page1, layer1) = PdfDocument::new("Network Observatory Report", Mm(210.0), Mm(297.0), "Layer 1");
    let layer = doc.get_page(page1).get_layer(layer1);
    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;

    let mut y: f32 = 280.0;
    let line = |layer: &PdfLayerReference, text: &str, size: f32, y: f32, font: &IndirectFontRef| {
        layer.use_text(text, size, Mm(15.0), Mm(y), font);
    };

    line(&layer, "Network Observatory - Report", 18.0, y, &font_bold);
    y -= 10.0;
    line(&layer, &format!("Generated at {}", input.generated_at), 10.0, y, &font);
    y -= 12.0;

    let (avg_score, worst) = overall_health_summary(&input.health);
    let up_count = input.interfaces.iter().filter(|i| format!("{:?}", i.oper_status) == "Up").count();

    line(&layer, "Executive Summary", 14.0, y, &font_bold);
    y -= 8.0;
    line(
        &layer,
        &format!("{} interfaces observed, {} currently up", input.interfaces.len(), up_count),
        10.0,
        y,
        &font,
    );
    y -= 6.0;
    line(&layer, &format!("Average health score: {avg_score:.1}/100"), 10.0, y, &font);
    y -= 6.0;
    if let Some(w) = worst {
        line(&layer, &format!("Least healthy interface: {} ({:.1}/100)", w.if_index, w.score), 10.0, y, &font);
        y -= 6.0;
    }
    line(&layer, &format!("{} alerts in this report window", input.alerts.len()), 10.0, y, &font);
    y -= 12.0;

    line(&layer, "Metrics", 14.0, y, &font_bold);
    y -= 8.0;
    for iface in input.interfaces.iter().take(20) {
        if y < 20.0 {
            break;
        }
        line(
            &layer,
            &format!(
                "#{} {} [{:?}] rx={} tx={} errors={} drops={}",
                iface.index,
                iface.name,
                iface.oper_status,
                iface.rx_bytes,
                iface.tx_bytes,
                iface.rx_errors + iface.tx_errors,
                iface.rx_drops + iface.tx_drops
            ),
            9.0,
            y,
            &font,
        );
        y -= 5.5;
    }

    y -= 8.0;
    if y > 20.0 {
        line(&layer, "Conclusions", 14.0, y, &font_bold);
        y -= 8.0;
        let conclusion = if avg_score >= 90.0 {
            "Overall network health is good across the observed interfaces."
        } else if avg_score >= 70.0 {
            "Overall network health shows some degradation - review alerts and least-healthy interfaces."
        } else {
            "Overall network health is poor - multiple interfaces show instability, packet loss, or downtime."
        };
        line(&layer, conclusion, 10.0, y, &font);
    }

    doc.save_to_bytes().map_err(|e| ReportError::Pdf(e.to_string()))
}

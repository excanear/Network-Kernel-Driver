use crate::ReportInput;

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

pub fn render(input: &ReportInput) -> Vec<u8> {
    let mut out = String::new();
    out.push_str("index,name,oper_status,link_speed_bps,mac_address,rx_bytes,tx_bytes,rx_errors,tx_errors,rx_drops,tx_drops\n");
    for iface in &input.interfaces {
        out.push_str(&format!(
            "{},{},{:?},{},{},{},{},{},{},{},{}\n",
            iface.index,
            csv_escape(&iface.name),
            iface.oper_status,
            iface.link_speed_bps.unwrap_or(0),
            csv_escape(&iface.mac_address),
            iface.rx_bytes,
            iface.tx_bytes,
            iface.rx_errors,
            iface.tx_errors,
            iface.rx_drops,
            iface.tx_drops,
        ));
    }
    out.into_bytes()
}

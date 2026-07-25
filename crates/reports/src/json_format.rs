use crate::ReportInput;

pub fn render(input: &ReportInput) -> Vec<u8> {
    serde_json::to_vec_pretty(input).unwrap_or_default()
}

#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    #[error("platform API call failed: {0}")]
    PlatformApi(String),

    #[error("failed to parse system data source: {0}")]
    Parse(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

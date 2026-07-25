mod error;
mod model;

pub use error::CollectorError;
pub use model::{CollectorSource, Duplex, InterfaceStats, OperStatus, Snapshot};

/// Platform-agnostic contract for producing a point-in-time view of all
/// network interfaces. Implemented today by `collector-windows` (IP Helper
/// API) and `collector-linux` (procfs/sysfs); Phase 2 adds a `collector-kernel`
/// backend talking to the real kernel-mode driver without changing this trait
/// or the `Snapshot`/`InterfaceStats` shape it returns.
pub trait InterfaceCollector: Send + Sync {
    fn snapshot(&self) -> Result<Snapshot, CollectorError>;
    fn platform_name(&self) -> &'static str;
}

//! Process-based plugin protocol for Network Observatory.
//!
//! Plugins are standalone executables (any language) that speak JSON over
//! stdout — no dynamic-library ABI to keep stable across Rust compiler
//! versions, no unsafe `dlopen`. A plugin is invoked as:
//!
//! - `<plugin> manifest` — prints a [`PluginManifest`] to stdout and exits.
//! - `<plugin> run --target <target>` — prints a [`PluginResult`] to stdout
//!   and exits. `target` is plugin-defined (e.g. a host to probe).
//!
//! The service (`crates/service/src/plugin_routes.rs`) discovers plugin
//! executables in a configured directory, calls `manifest` once at startup,
//! and calls `run` on demand via `GET /api/v1/plugins/:name/run`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginKind {
    Collector,
    Alert,
    Report,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetric {
    pub key: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResult {
    pub plugin: String,
    pub target: String,
    pub metrics: Vec<PluginMetric>,
    pub collected_at: DateTime<Utc>,
    pub error: Option<String>,
}

pub const MANIFEST_ARG: &str = "manifest";
pub const RUN_ARG: &str = "run";

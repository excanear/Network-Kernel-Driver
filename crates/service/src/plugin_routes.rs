use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use plugin_api::{PluginManifest, PluginResult, MANIFEST_ARG, RUN_ARG};
use serde::Deserialize;

use crate::routes::AppState;

/// Plugin executables are discovered next to the running binary (the same
/// `target/debug` or `target/release` directory cargo places all workspace
/// binaries into) plus an optional operator-configured directory via
/// `NETOBS_PLUGINS_DIR` — so production deployments can drop extra plugin
/// executables into a dedicated folder without rebuilding the workspace.
fn plugin_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
        }
    }
    if let Ok(extra) = std::env::var("NETOBS_PLUGINS_DIR") {
        dirs.push(PathBuf::from(extra));
    }
    dirs
}

const KNOWN_PLUGIN_BINARIES: &[&str] = &["ping-latency-collector"];

fn plugin_binary_path(name: &str) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    plugin_search_dirs().into_iter().map(|d| d.join(&exe_name)).find(|p| p.exists())
}

pub async fn list_plugins() -> Json<Vec<PluginManifest>> {
    let mut manifests = Vec::new();
    for name in KNOWN_PLUGIN_BINARIES {
        let Some(path) = plugin_binary_path(name) else { continue };
        let Ok(output) = Command::new(&path).arg(MANIFEST_ARG).output() else { continue };
        if let Ok(manifest) = serde_json::from_slice::<PluginManifest>(&output.stdout) {
            manifests.push(manifest);
        }
    }
    Json(manifests)
}

#[derive(Deserialize)]
pub struct RunQuery {
    pub target: Option<String>,
}

pub async fn run_plugin(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(query): Query<RunQuery>,
) -> Result<Json<PluginResult>, (StatusCode, String)> {
    let path = plugin_binary_path(&name)
        .ok_or((StatusCode::NOT_FOUND, format!("plugin '{name}' not found")))?;

    let mut cmd = Command::new(&path);
    cmd.arg(RUN_ARG);
    if let Some(target) = &query.target {
        cmd.arg("--target").arg(target);
    }

    let output = cmd
        .output()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to spawn plugin: {e}")))?;

    let result = serde_json::from_slice::<PluginResult>(&output.stdout)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("invalid plugin output: {e}")))?;

    let _ = state.audit_store.record(
        "plugin.run",
        "system",
        &format!("plugin={name} target={}", query.target.as_deref().unwrap_or("default")),
    );

    Ok(Json(result))
}

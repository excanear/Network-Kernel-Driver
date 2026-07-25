pub mod history;
pub mod interfaces;
pub mod monitor;
pub mod status;

pub fn not_implemented(command: &str) {
    println!(
        "`network {command}` is not implemented in this Phase 1 slice yet.\nSee docs/roadmap.md for the planned scope."
    );
}

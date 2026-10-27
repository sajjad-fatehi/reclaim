//! Native read-only scans and explicitly confirmed cleanup. No generic frontend filesystem or shell capability.

pub mod scan;

pub mod cleanup;
mod cleanup_history;
pub mod docker;
pub mod preferences;

#[cfg(test)]
pub(crate) fn test_context() -> tauri::Context<tauri::test::MockRuntime> {
    tauri::generate_context!()
}

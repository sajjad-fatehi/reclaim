//! Native read-only scan producer. No frontend filesystem or shell capability.

pub mod scan;

pub mod cleanup;
mod cleanup_history;
pub mod docker;
pub mod preferences;

#[cfg(test)]
pub(crate) fn test_context() -> tauri::Context<tauri::test::MockRuntime> {
    tauri::generate_context!()
}

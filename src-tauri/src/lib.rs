//! Native read-only scan producer. No frontend filesystem or shell capability.

pub mod scan;

pub mod docker;

#[cfg(test)]
pub(crate) fn test_context() -> tauri::Context<tauri::test::MockRuntime> {
    tauri::generate_context!()
}

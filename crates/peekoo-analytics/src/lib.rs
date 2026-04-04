/// Analytics event definitions and provider integration for Peekoo.
///
/// This crate owns event names, property construction, and provider
/// configuration. Transport-specific wiring (e.g. Tauri plugin registration)
/// lives in the consuming crate.
pub mod events;
pub mod posthog;

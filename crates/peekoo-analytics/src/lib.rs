/// Analytics event definitions and provider integration for Peekoo.
///
/// This crate owns event names, property construction, and provider
/// initialisation. Transport-specific wiring (e.g. Tauri plugin registration)
/// lives in the consuming crate.
///
/// Enable features to pull in provider dependencies:
/// - `sentry` -- Sentry error tracking
pub mod events;
pub mod posthog;
#[cfg(feature = "sentry")]
pub mod sentry;

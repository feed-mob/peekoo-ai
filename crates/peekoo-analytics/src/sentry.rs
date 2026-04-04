//! Sentry error tracking configuration and initialisation.
//!
//! Owns the Sentry DSN, client options, and `init()` call. The caller is
//! responsible for Tauri plugin registration.
//!
//! The DSN is read from the `SENTRY_DSN` environment variable at **compile
//! time** via `option_env!`. When absent (local dev, open-source forks),
//! [`init`] returns `None` and error tracking is disabled.

use std::sync::OnceLock;

/// Re-export the Sentry `Client` and `ClientInitGuard` types so callers
/// don't need a direct `sentry` dependency.
pub use sentry::{Client, ClientInitGuard};

/// Global Sentry guard. Stored in a static so the guard (and its `Client`)
/// live for the entire process, which satisfies the `'static` lifetime
/// required by Tauri plugin registration.
static GUARD: OnceLock<Option<ClientInitGuard>> = OnceLock::new();

/// Initialise the Sentry SDK if the `SENTRY_DSN` env var was set at compile
/// time.
///
/// Returns `true` when Sentry is enabled, `false` otherwise. Must be called
/// **once** at app startup, before the Tauri builder. Subsequent calls are
/// no-ops.
pub fn init() -> bool {
    let guard = GUARD.get_or_init(|| {
        let dsn = option_env!("SENTRY_DSN")?;
        if dsn.is_empty() {
            return None;
        }
        let guard = sentry::init((
            dsn.to_string(),
            sentry::ClientOptions {
                release: sentry::release_name!(),
                ..Default::default()
            },
        ));
        if guard.is_enabled() {
            Some(guard)
        } else {
            None
        }
    });
    guard.is_some()
}

/// Return a `&'static Client` reference for Tauri plugin registration.
///
/// If Sentry was initialised via [`init`], returns the real client.
/// Otherwise returns a disabled dummy client.
pub fn client() -> &'static Client {
    static DUMMY: OnceLock<Client> = OnceLock::new();

    GUARD
        .get()
        .and_then(|opt| opt.as_ref())
        .map(|guard| &**guard)
        .unwrap_or_else(|| DUMMY.get_or_init(|| Client::from(sentry::ClientOptions::default())))
}

/// Return a reference to the guard if Sentry is active. Needed for
/// minidump initialisation which requires `&ClientInitGuard`.
pub fn guard() -> Option<&'static ClientInitGuard> {
    GUARD.get().and_then(|opt| opt.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_returns_disabled_when_no_dsn() {
        // In test builds SENTRY_DSN is not set, so client should be disabled.
        assert!(!client().is_enabled());
    }

    #[test]
    fn client_returns_same_instance() {
        let a = client() as *const Client;
        let b = client() as *const Client;
        assert_eq!(a, b);
    }

    #[test]
    fn guard_is_none_when_no_dsn() {
        assert!(guard().is_none());
    }
}

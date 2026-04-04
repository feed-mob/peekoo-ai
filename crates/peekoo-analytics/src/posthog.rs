//! PostHog provider configuration.
//!
//! Holds the API key and host URL. Transport-specific wiring (Tauri plugin
//! registration) is the caller's responsibility.
//!
//! The API key is read from the `POSTHOG_API_KEY` environment variable at
//! **compile time** via `option_env!`. When absent (local dev, open-source
//! forks), [`config_from_env`] returns `None` and analytics should be skipped.

const DEFAULT_API_HOST: &str = "https://us.i.posthog.com";

/// Provider-owned capture payload that transport layers can map onto their
/// specific client SDK request types.
#[derive(Debug, Clone)]
pub struct PostHogCapture {
    event: String,
    properties: std::collections::HashMap<String, serde_json::Value>,
}

/// Configuration needed to initialise a PostHog analytics provider.
#[derive(Debug, Clone)]
pub struct PostHogAnalyticsConfig {
    api_key: String,
    api_host: String,
}

/// Return a config if the `POSTHOG_API_KEY` env var was set at compile time.
///
/// Optionally reads `POSTHOG_API_HOST` for the ingest host URL, falling back
/// to the default US cloud host when absent.
///
/// Returns `None` for local dev builds and open-source forks where the key
/// is not available.
pub fn config_from_env() -> Option<PostHogAnalyticsConfig> {
    let api_key = option_env!("POSTHOG_API_KEY")?;
    if api_key.is_empty() {
        return None;
    }
    let config = match option_env!("POSTHOG_API_HOST") {
        Some(host) if !host.is_empty() => PostHogAnalyticsConfig::with_host(api_key, host),
        _ => PostHogAnalyticsConfig::new(api_key),
    };
    Some(config)
}

impl PostHogAnalyticsConfig {
    /// Create a config with the default US cloud host.
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            api_host: DEFAULT_API_HOST.to_string(),
        }
    }

    /// Create a config with a custom host (e.g. EU cloud or self-hosted).
    pub fn with_host(api_key: &str, api_host: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            api_host: api_host.to_string(),
        }
    }

    /// The PostHog project API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// The PostHog ingest host URL.
    pub fn api_host(&self) -> &str {
        &self.api_host
    }
}

impl PostHogCapture {
    /// The event name to send to PostHog.
    pub fn event(&self) -> &str {
        &self.event
    }

    /// Event properties to include with the capture request.
    pub fn properties(&self) -> &std::collections::HashMap<String, serde_json::Value> {
        &self.properties
    }
}

/// Build the standard PostHog payload for the app startup event.
pub fn app_started_capture(version: &str, os: &str, arch: &str) -> PostHogCapture {
    PostHogCapture {
        event: crate::events::APP_STARTED.to_string(),
        properties: crate::events::app_started_properties(version, os, arch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_provided_api_key() {
        let config = PostHogAnalyticsConfig::new("phc_test_key");
        assert_eq!(config.api_key(), "phc_test_key");
    }

    #[test]
    fn default_config_uses_us_host() {
        let config = PostHogAnalyticsConfig::new("phc_test_key");
        assert_eq!(config.api_host(), "https://us.i.posthog.com");
    }

    #[test]
    fn custom_host_overrides_default() {
        let config = PostHogAnalyticsConfig::with_host("phc_test_key", "https://eu.posthog.com");
        assert_eq!(config.api_host(), "https://eu.posthog.com");
    }

    #[test]
    fn app_started_capture_uses_app_started_event() {
        let capture = app_started_capture("0.1.21", "linux", "x86_64");
        assert_eq!(capture.event(), crate::events::APP_STARTED);
    }

    #[test]
    fn app_started_capture_includes_expected_properties() {
        let capture = app_started_capture("0.1.21", "linux", "x86_64");
        assert_eq!(capture.properties().len(), 3);
        assert_eq!(
            capture.properties().get("app_version"),
            Some(&serde_json::json!("0.1.21"))
        );
        assert_eq!(
            capture.properties().get("os"),
            Some(&serde_json::json!("linux"))
        );
        assert_eq!(
            capture.properties().get("arch"),
            Some(&serde_json::json!("x86_64"))
        );
    }
}

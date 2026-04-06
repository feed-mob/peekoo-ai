//! PostHog provider configuration.
//!
//! Holds the API key and host URL. Transport-specific wiring (Tauri plugin
//! registration) is the caller's responsibility.
//!
//! The API key is read from the `POSTHOG_API_KEY` environment variable at
//! **compile time** via `option_env!`. When absent (local dev, open-source
//! forks), [`config_from_env`] returns `None` and analytics should be skipped.

const DEFAULT_API_HOST: &str = "https://us.i.posthog.com";

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
    fn config_from_env_matches_compile_time_key_presence() {
        let config = config_from_env();
        match option_env!("POSTHOG_API_KEY") {
            Some(api_key) if !api_key.is_empty() => {
                let config = config.expect("config should be Some when POSTHOG_API_KEY is set");
                assert_eq!(config.api_key(), api_key);
            }
            _ => {
                assert!(
                    config.is_none(),
                    "config should be None when POSTHOG_API_KEY is unset or empty"
                );
            }
        }
    }
}

//! Main registry client for fetching and caching ACP registry

use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;

use crate::cache::Cache;
use crate::types::Registry;

/// ACP Registry CDN URL
pub const REGISTRY_URL: &str =
    "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";

/// Default cache TTL: 1 hour (3600 seconds)
pub const CACHE_TTL_SECONDS: u64 = 3600;

/// Client for fetching and caching ACP registry
///
/// Automatically handles:
/// - Fetching from CDN
/// - Local file-based caching
/// - TTL-based refresh
/// - Offline fallback (use cached data)
#[derive(Clone)]
pub struct RegistryClient {
    http: Client,
    cache: Cache,
    ttl_seconds: u64,
    registry_url: String,
}

impl RegistryClient {
    /// Create new client with default cache location and TTL
    pub fn new() -> Result<Self> {
        Self::with_cache_dir_and_ttl(None, CACHE_TTL_SECONDS)
    }

    /// Create with custom TTL
    pub fn with_ttl(ttl_seconds: u64) -> Result<Self> {
        Self::with_cache_dir_and_ttl(None, ttl_seconds)
    }

    /// Create with custom cache directory
    pub fn with_cache_dir(cache_dir: PathBuf) -> Result<Self> {
        Self::with_cache_dir_and_ttl(Some(cache_dir), CACHE_TTL_SECONDS)
    }

    /// Create with custom cache directory and TTL
    pub fn with_cache_dir_and_ttl(cache_dir: Option<PathBuf>, ttl_seconds: u64) -> Result<Self> {
        Self::with_options(cache_dir, ttl_seconds, REGISTRY_URL.to_string())
    }

    /// Create with full customization including registry URL
    pub fn with_options(
        cache_dir: Option<PathBuf>,
        ttl_seconds: u64,
        registry_url: String,
    ) -> Result<Self> {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .context("Failed to build HTTP client")?;

        let cache = match cache_dir {
            Some(dir) => Cache::with_dir(dir)?,
            None => Cache::new()?,
        };

        Ok(Self {
            http,
            cache,
            ttl_seconds,
            registry_url,
        })
    }

    /// Fetch registry (cached if fresh, otherwise from CDN)
    ///
    /// On network failure, returns cached data if available
    pub async fn fetch(&self) -> Result<Registry> {
        // Check if cache is fresh
        let is_stale = self.cache.is_stale(self.ttl_seconds).await.unwrap_or(true);

        if !is_stale {
            // Use cached data
            if let Some(registry) = self.cache.load_registry().await? {
                return Ok(registry);
            }
        }

        // Try to refresh from CDN
        match self.refresh().await {
            Ok(registry) => Ok(registry),
            Err(e) => {
                // Network failed, try to use stale cache
                if let Some(registry) = self.cache.load_registry().await? {
                    // Silently use stale data (as per preference)
                    Ok(registry)
                } else {
                    // No cache available, return error
                    Err(e)
                }
            }
        }
    }

    /// Force refresh from CDN
    ///
    /// Downloads fresh registry data and updates cache
    pub async fn refresh(&self) -> Result<Registry> {
        // Fetch from CDN
        let response = self
            .http
            .get(&self.registry_url)
            .send()
            .await
            .context("Failed to fetch registry from CDN")?;

        // Check status
        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Registry CDN returned error status: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            ));
        }

        // Parse JSON
        let registry: Registry = response
            .json()
            .await
            .context("Failed to parse registry JSON")?;

        // Save to cache
        self.cache
            .save_registry(&registry)
            .await
            .context("Failed to save registry to cache")?;

        Ok(registry)
    }

    /// Get last fetch timestamp
    pub async fn last_updated(&self) -> Result<Option<SystemTime>> {
        let cached_at = self.cache.cached_at().await?;
        Ok(cached_at.map(|dt| dt.into()))
    }

    /// Get last fetch timestamp as DateTime
    pub async fn last_updated_datetime(&self) -> Result<Option<DateTime<Utc>>> {
        self.cache.cached_at().await
    }

    /// Check if cache is stale (older than TTL)
    pub async fn is_stale(&self) -> Result<bool> {
        self.cache.is_stale(self.ttl_seconds).await
    }

    /// Get cache directory path
    pub fn cache_dir(&self) -> &std::path::Path {
        self.cache
            .registry_path()
            .parent()
            .unwrap_or(self.cache.registry_path())
    }

    /// Get the TTL in seconds
    pub fn ttl_seconds(&self) -> u64 {
        self.ttl_seconds
    }

    /// Set new TTL
    pub fn set_ttl(&mut self, ttl_seconds: u64) {
        self.ttl_seconds = ttl_seconds;
    }
}

impl Default for RegistryClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default RegistryClient")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_client_fetch_and_cache() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Create test registry JSON
        let registry_json = serde_json::json!({
            "version": "1.0.0",
            "agents": [],
            "extensions": []
        });

        // Mock the registry endpoint
        Mock::given(method("GET"))
            .and(path("/registry.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&registry_json))
            .mount(&mock_server)
            .await;

        // Create temp cache with mock server URL
        let temp_dir = tempfile::tempdir().unwrap();
        let client = RegistryClient::with_options(
            Some(temp_dir.path().to_path_buf()),
            CACHE_TTL_SECONDS,
            mock_server.uri() + "/registry.json",
        )
        .unwrap();

        // First fetch should hit the server
        let registry = client.fetch().await.unwrap();
        assert_eq!(registry.version, "1.0.0");

        // Verify cache was created
        assert!(client.cache.registry_path().exists());
    }

    #[tokio::test]
    async fn test_client_offline_fallback() {
        // Create temp cache with pre-existing data
        let temp_dir = tempfile::tempdir().unwrap();
        let cache = Cache::with_dir(temp_dir.path().to_path_buf()).unwrap();

        // Pre-populate cache
        let cached_registry = Registry {
            version: "cached-1.0.0".to_string(),
            agents: vec![],
            extensions: vec![],
        };
        cache.save_registry(&cached_registry).await.unwrap();

        // Create client pointing to non-existent server
        let client = RegistryClient::with_cache_dir(temp_dir.path().to_path_buf()).unwrap();

        // Fetch should fallback to cache
        let registry = client.fetch().await.unwrap();
        assert_eq!(registry.version, "cached-1.0.0");
    }

    #[tokio::test]
    async fn test_client_refresh_forces_update() {
        let mock_server = MockServer::start().await;

        let registry_json = serde_json::json!({
            "version": "2.0.0",
            "agents": [],
            "extensions": []
        });

        Mock::given(method("GET"))
            .and(path("/registry.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&registry_json))
            .mount(&mock_server)
            .await;

        let temp_dir = tempfile::tempdir().unwrap();
        let client = RegistryClient::with_options(
            Some(temp_dir.path().to_path_buf()),
            CACHE_TTL_SECONDS,
            mock_server.uri() + "/registry.json",
        )
        .unwrap();

        // Refresh should always fetch from server
        let registry = client.refresh().await.unwrap();
        assert_eq!(registry.version, "2.0.0");
    }
}

//! File-based cache for registry data and agent icons
//!
//! Cache locations:
//! - Registry: `~/.peekoo/cache/acp-registry.json`
//! - Icons: `~/.peekoo/cache/icons/<agent-id>.svg`
//! - Metadata: `~/.peekoo/cache/registry-meta.json` (timestamp, version)

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::types::Registry;

/// Cache metadata for tracking freshness
#[derive(Debug, Clone, Deserialize, Serialize)]
struct CacheMetadata {
    cached_at: DateTime<Utc>,
    registry_version: String,
    etag: Option<String>, // For future HTTP conditional requests
}

/// File-based cache for registry data
pub struct Cache {
    cache_dir: PathBuf,
    registry_file: PathBuf,
    icons_dir: PathBuf,
    metadata_file: PathBuf,
}

// Manual Clone implementation since PathBuf implements Clone
impl Clone for Cache {
    fn clone(&self) -> Self {
        Self {
            cache_dir: self.cache_dir.clone(),
            registry_file: self.registry_file.clone(),
            icons_dir: self.icons_dir.clone(),
            metadata_file: self.metadata_file.clone(),
        }
    }
}

impl Cache {
    /// Create cache with default location
    pub fn new() -> Result<Self> {
        let cache_dir = peekoo_paths::peekoo_global_cache_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get cache dir: {}", e))?;
        Self::with_dir(cache_dir)
    }

    /// Create cache with custom directory
    pub fn with_dir(cache_dir: PathBuf) -> Result<Self> {
        let registry_file = cache_dir.join("acp-registry.json");
        let icons_dir = cache_dir.join("icons");
        let metadata_file = cache_dir.join("registry-meta.json");

        Ok(Self {
            cache_dir,
            registry_file,
            icons_dir,
            metadata_file,
        })
    }

    /// Ensure cache directories exist
    async fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.cache_dir)
            .await
            .context("Failed to create cache directory")?;
        fs::create_dir_all(&self.icons_dir)
            .await
            .context("Failed to create icons directory")?;
        Ok(())
    }

    /// Save registry to cache
    pub async fn save_registry(&self, registry: &Registry) -> Result<()> {
        self.ensure_dirs().await?;

        // Serialize registry
        let json =
            serde_json::to_string_pretty(registry).context("Failed to serialize registry")?;

        // Write atomically (write to temp, then rename)
        let temp_file = self.registry_file.with_extension("tmp");
        fs::write(&temp_file, json)
            .await
            .context("Failed to write registry cache")?;
        fs::rename(&temp_file, &self.registry_file)
            .await
            .context("Failed to finalize registry cache")?;

        // Save metadata
        let metadata = CacheMetadata {
            cached_at: Utc::now(),
            registry_version: registry.version.clone(),
            etag: None,
        };
        let meta_json =
            serde_json::to_string(&metadata).context("Failed to serialize cache metadata")?;
        fs::write(&self.metadata_file, meta_json)
            .await
            .context("Failed to write cache metadata")?;

        Ok(())
    }

    /// Load registry from cache
    pub async fn load_registry(&self) -> Result<Option<Registry>> {
        // Check if cache exists
        if !self.registry_file.exists() {
            return Ok(None);
        }

        // Read and parse
        let json = fs::read_to_string(&self.registry_file)
            .await
            .context("Failed to read registry cache")?;
        let registry = serde_json::from_str(&json).context("Failed to parse registry cache")?;

        Ok(Some(registry))
    }

    /// Get cache timestamp
    pub async fn cached_at(&self) -> Result<Option<DateTime<Utc>>> {
        if !self.metadata_file.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&self.metadata_file)
            .await
            .context("Failed to read cache metadata")?;
        let metadata: CacheMetadata =
            serde_json::from_str(&json).context("Failed to parse cache metadata")?;

        Ok(Some(metadata.cached_at))
    }

    /// Check if cache is stale (older than TTL)
    pub async fn is_stale(&self, ttl_seconds: u64) -> Result<bool> {
        let cached_at = match self.cached_at().await? {
            Some(ts) => ts,
            None => return Ok(true), // No cache = stale
        };

        let age = Utc::now() - cached_at;
        let ttl = chrono::Duration::seconds(ttl_seconds as i64);

        Ok(age > ttl)
    }

    /// Get registry cache file path
    pub fn registry_path(&self) -> &Path {
        &self.registry_file
    }

    /// Save agent icon
    pub async fn save_icon(&self, agent_id: &str, svg_data: &[u8]) -> Result<PathBuf> {
        self.ensure_dirs().await?;

        let icon_path = self.icons_dir.join(format!("{}.svg", agent_id));
        fs::write(&icon_path, svg_data)
            .await
            .context("Failed to write icon")?;

        Ok(icon_path)
    }

    /// Load agent icon
    pub async fn load_icon(&self, agent_id: &str) -> Result<Option<Vec<u8>>> {
        let icon_path = self.icons_dir.join(format!("{}.svg", agent_id));

        if !icon_path.exists() {
            return Ok(None);
        }

        let data = fs::read(&icon_path).await.context("Failed to read icon")?;

        Ok(Some(data))
    }

    /// Get icon path (may not exist)
    pub fn icon_path(&self, agent_id: &str) -> PathBuf {
        self.icons_dir.join(format!("{}.svg", agent_id))
    }

    /// Clear all cache
    pub async fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .await
                .context("Failed to clear cache")?;
        }
        Ok(())
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new().expect("Failed to create default cache")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_cache_save_load_registry() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache::with_dir(temp_dir.path().to_path_buf()).unwrap();

        let registry = Registry {
            version: "1.0.0".to_string(),
            agents: vec![],
            extensions: vec![],
        };

        // Save
        cache.save_registry(&registry).await.unwrap();

        // Load
        let loaded = cache.load_registry().await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().version, "1.0.0");
    }

    #[tokio::test]
    async fn test_cache_stale_check() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache::with_dir(temp_dir.path().to_path_buf()).unwrap();

        let registry = Registry {
            version: "1.0.0".to_string(),
            agents: vec![],
            extensions: vec![],
        };

        // Save
        cache.save_registry(&registry).await.unwrap();

        // Check freshness with long TTL
        assert!(!cache.is_stale(3600).await.unwrap());

        // Check freshness with 0 TTL (should be stale)
        assert!(cache.is_stale(0).await.unwrap());
    }

    #[tokio::test]
    async fn test_cache_save_load_icon() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache::with_dir(temp_dir.path().to_path_buf()).unwrap();

        let svg_data = b"<svg></svg>";
        let path = cache.save_icon("test-agent", svg_data).await.unwrap();

        assert!(path.exists());

        let loaded = cache.load_icon("test-agent").await.unwrap();
        assert_eq!(loaded.unwrap(), svg_data);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Cache::with_dir(temp_dir.path().to_path_buf()).unwrap();

        let registry = Registry {
            version: "1.0.0".to_string(),
            agents: vec![],
            extensions: vec![],
        };

        cache.save_registry(&registry).await.unwrap();
        assert!(cache.registry_file.exists());

        cache.clear().await.unwrap();
        assert!(!cache.cache_dir.exists());
    }
}

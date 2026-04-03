//! HTTP client wrapper using reqwest
//! Replaces Zed's custom http_client crate

use anyhow::Result;
use bytes::Bytes;

/// Simple HTTP client wrapper around reqwest
#[derive(Clone)]
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    /// Create a new HTTP client with default settings
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    /// Create a blocked HTTP client that always fails
    /// Used for unavailable runtime
    pub fn blocked() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(1))
                .build()
                .expect("Failed to build blocked HTTP client"),
        }
    }

    /// Perform a GET request and return the response body
    pub async fn get(&self, url: &str) -> Result<Bytes> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send HTTP request: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "HTTP request failed with status: {}",
                status
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;
        Ok(bytes)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

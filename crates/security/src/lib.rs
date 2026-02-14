use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SecretStoreError {
    #[error("secret not found")]
    NotFound,
    #[error("secret store unavailable")]
    Unavailable,
}

pub trait SecretStore: Send + Sync {
    fn put(&self, key: &str, value: &str) -> Result<(), SecretStoreError>;
    fn get(&self, key: &str) -> Result<String, SecretStoreError>;
}

#[derive(Clone, Default)]
pub struct InMemorySecretStore {
    inner: Arc<Mutex<HashMap<String, String>>>,
}

impl SecretStore for InMemorySecretStore {
    fn put(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        let mut lock = self
            .inner
            .lock()
            .map_err(|_| SecretStoreError::Unavailable)?;
        lock.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<String, SecretStoreError> {
        let lock = self
            .inner
            .lock()
            .map_err(|_| SecretStoreError::Unavailable)?;
        lock.get(key).cloned().ok_or(SecretStoreError::NotFound)
    }
}

pub fn redact_secret(input: &str) -> String {
    if input.len() <= 8 {
        return "[REDACTED]".to_string();
    }
    let suffix = &input[input.len() - 4..];
    format!("[REDACTED-{}]", suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_then_get_secret() {
        let store = InMemorySecretStore::default();
        store
            .put("token_ref", "super-secret-token")
            .expect("store secret");
        let value = store.get("token_ref").expect("read secret");
        assert_eq!(value, "super-secret-token");
    }

    #[test]
    fn redact_secret_keeps_only_suffix() {
        let redacted = redact_secret("abcdefghijklmnop");
        assert_eq!(redacted, "[REDACTED-mnop]");
    }
}

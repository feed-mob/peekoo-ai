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
    fn delete(&self, key: &str) -> Result<(), SecretStoreError>;
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

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        let mut lock = self
            .inner
            .lock()
            .map_err(|_| SecretStoreError::Unavailable)?;
        lock.remove(key)
            .map(|_| ())
            .ok_or(SecretStoreError::NotFound)
    }
}

#[derive(Clone)]
pub struct KeyringSecretStore {
    service: String,
}

impl KeyringSecretStore {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, SecretStoreError> {
        keyring::Entry::new(&self.service, key).map_err(|_| SecretStoreError::Unavailable)
    }
}

impl SecretStore for KeyringSecretStore {
    fn put(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        let entry = self.entry(key)?;
        entry
            .set_password(value)
            .map_err(|_| SecretStoreError::Unavailable)
    }

    fn get(&self, key: &str) -> Result<String, SecretStoreError> {
        let entry = self.entry(key)?;
        match entry.get_password() {
            Ok(v) => Ok(v),
            Err(keyring::Error::NoEntry) => Err(SecretStoreError::NotFound),
            Err(_) => Err(SecretStoreError::Unavailable),
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        let entry = self.entry(key)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Err(SecretStoreError::NotFound),
            Err(_) => Err(SecretStoreError::Unavailable),
        }
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

    #[test]
    fn delete_secret_removes_value() {
        let store = InMemorySecretStore::default();
        store.put("token_ref", "super-secret-token").expect("store");
        store.delete("token_ref").expect("delete");
        let read_after_delete = store.get("token_ref");
        assert_eq!(read_after_delete, Err(SecretStoreError::NotFound));
    }
}

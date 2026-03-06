use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

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

#[derive(Clone)]
pub struct FileSecretStore {
    root: PathBuf,
}

impl FileSecretStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn ensure_root_dir(&self) -> Result<(), SecretStoreError> {
        fs::create_dir_all(&self.root).map_err(|_| SecretStoreError::Unavailable)?;
        #[cfg(unix)]
        {
            let perms = fs::Permissions::from_mode(0o700);
            fs::set_permissions(&self.root, perms).map_err(|_| SecretStoreError::Unavailable)?;
        }
        Ok(())
    }

    fn path_for_key(&self, key: &str) -> PathBuf {
        self.root.join(format!("{}.secret", key_to_hex(key)))
    }
}

impl SecretStore for FileSecretStore {
    fn put(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        self.ensure_root_dir()?;
        let path = self.path_for_key(key);

        let mut opts = OpenOptions::new();
        opts.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            opts.mode(0o600);
        }

        let mut file = opts
            .open(&path)
            .map_err(|_| SecretStoreError::Unavailable)?;
        file.write_all(value.as_bytes())
            .map_err(|_| SecretStoreError::Unavailable)?;
        file.sync_all().map_err(|_| SecretStoreError::Unavailable)?;

        #[cfg(unix)]
        {
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms).map_err(|_| SecretStoreError::Unavailable)?;
        }

        Ok(())
    }

    fn get(&self, key: &str) -> Result<String, SecretStoreError> {
        let path = self.path_for_key(key);
        fs::read_to_string(&path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                SecretStoreError::NotFound
            } else {
                SecretStoreError::Unavailable
            }
        })
    }

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        let path = self.path_for_key(key);
        fs::remove_file(&path).map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                SecretStoreError::NotFound
            } else {
                SecretStoreError::Unavailable
            }
        })
    }
}

pub struct FallbackSecretStore {
    primary: Box<dyn SecretStore>,
    fallback: Box<dyn SecretStore>,
}

impl FallbackSecretStore {
    pub fn new(primary: Box<dyn SecretStore>, fallback: Box<dyn SecretStore>) -> Self {
        Self { primary, fallback }
    }
}

impl SecretStore for FallbackSecretStore {
    fn put(&self, key: &str, value: &str) -> Result<(), SecretStoreError> {
        match self.primary.put(key, value) {
            Ok(()) => match self.primary.get(key) {
                Ok(_) => {
                    let _ = self.fallback.delete(key);
                    Ok(())
                }
                Err(SecretStoreError::Unavailable) | Err(SecretStoreError::NotFound) => {
                    self.fallback.put(key, value)
                }
            },
            Err(SecretStoreError::Unavailable) => self.fallback.put(key, value),
            Err(SecretStoreError::NotFound) => self.fallback.put(key, value),
        }
    }

    fn get(&self, key: &str) -> Result<String, SecretStoreError> {
        match self.primary.get(key) {
            Ok(value) => Ok(value),
            Err(SecretStoreError::Unavailable) | Err(SecretStoreError::NotFound) => {
                self.fallback.get(key)
            }
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecretStoreError> {
        let primary = self.primary.delete(key);
        let fallback = self.fallback.delete(key);

        match (primary, fallback) {
            (Ok(()), _) | (_, Ok(())) => Ok(()),
            (Err(SecretStoreError::NotFound), Err(SecretStoreError::NotFound)) => {
                Err(SecretStoreError::NotFound)
            }
            _ => Err(SecretStoreError::Unavailable),
        }
    }
}

fn key_to_hex(key: &str) -> String {
    key.as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join("")
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

    struct UnavailableSecretStore;

    impl SecretStore for UnavailableSecretStore {
        fn put(&self, _key: &str, _value: &str) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }

        fn get(&self, _key: &str) -> Result<String, SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }

        fn delete(&self, _key: &str) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }
    }

    struct WriteOnlySecretStore;

    impl SecretStore for WriteOnlySecretStore {
        fn put(&self, _key: &str, _value: &str) -> Result<(), SecretStoreError> {
            Ok(())
        }

        fn get(&self, _key: &str) -> Result<String, SecretStoreError> {
            Err(SecretStoreError::NotFound)
        }

        fn delete(&self, _key: &str) -> Result<(), SecretStoreError> {
            Ok(())
        }
    }

    fn temp_secrets_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "peekoo-security-{prefix}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

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

    #[test]
    fn file_secret_store_round_trip() {
        let root = temp_secrets_dir("file-round-trip");
        let store = FileSecretStore::new(root.clone());

        store
            .put("peekoo/auth/demo/api-key", "secret-value")
            .expect("put");
        let value = store.get("peekoo/auth/demo/api-key").expect("get");
        assert_eq!(value, "secret-value");

        store.delete("peekoo/auth/demo/api-key").expect("delete");
        assert_eq!(
            store.get("peekoo/auth/demo/api-key"),
            Err(SecretStoreError::NotFound)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fallback_put_uses_secondary_store_when_primary_unavailable() {
        let fallback_mem = InMemorySecretStore::default();
        let store = FallbackSecretStore::new(
            Box::new(UnavailableSecretStore),
            Box::new(fallback_mem.clone()),
        );

        store
            .put("peekoo/auth/openai/api-key", "value")
            .expect("put");
        let value = store.get("peekoo/auth/openai/api-key").expect("get");
        assert_eq!(value, "value");
        assert_eq!(
            fallback_mem.get("peekoo/auth/openai/api-key").as_deref(),
            Ok("value")
        );
    }

    #[test]
    fn fallback_put_uses_secondary_when_primary_write_not_readable() {
        let fallback_mem = InMemorySecretStore::default();
        let store = FallbackSecretStore::new(
            Box::new(WriteOnlySecretStore),
            Box::new(fallback_mem.clone()),
        );

        store
            .put("peekoo/auth/anthropic/api-key", "value")
            .expect("put through fallback");
        assert_eq!(
            fallback_mem.get("peekoo/auth/anthropic/api-key").as_deref(),
            Ok("value")
        );
    }

    #[test]
    fn fallback_get_prefers_primary_store() {
        let primary = InMemorySecretStore::default();
        let secondary = InMemorySecretStore::default();
        primary
            .put("peekoo/auth/test/api-key", "primary")
            .expect("seed primary");
        secondary
            .put("peekoo/auth/test/api-key", "secondary")
            .expect("seed secondary");

        let store = FallbackSecretStore::new(Box::new(primary), Box::new(secondary));
        let value = store.get("peekoo/auth/test/api-key").expect("read value");
        assert_eq!(value, "primary");
    }

    #[test]
    fn fallback_get_returns_not_found_when_missing_in_both() {
        let store = FallbackSecretStore::new(
            Box::new(InMemorySecretStore::default()),
            Box::new(InMemorySecretStore::default()),
        );
        assert_eq!(
            store.get("peekoo/auth/none/api-key"),
            Err(SecretStoreError::NotFound)
        );
    }

    #[test]
    fn fallback_delete_succeeds_when_deleted_in_secondary_only() {
        let secondary = InMemorySecretStore::default();
        secondary
            .put("peekoo/auth/fallback/api-key", "value")
            .expect("seed secondary");

        let store = FallbackSecretStore::new(
            Box::new(UnavailableSecretStore),
            Box::new(secondary.clone()),
        );
        store
            .delete("peekoo/auth/fallback/api-key")
            .expect("delete via fallback");
        assert_eq!(
            secondary.get("peekoo/auth/fallback/api-key"),
            Err(SecretStoreError::NotFound)
        );
    }

    #[cfg(unix)]
    #[test]
    fn file_secret_store_applies_restrictive_permissions() {
        let root = temp_secrets_dir("perms");
        let store = FileSecretStore::new(root.clone());
        let key = "peekoo/auth/perms/api-key";

        store.put(key, "secret").expect("put secret");
        let path = store.path_for_key(key);

        let root_mode = fs::metadata(&root)
            .expect("root metadata")
            .permissions()
            .mode()
            & 0o777;
        let file_mode = fs::metadata(&path)
            .expect("file metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(root_mode, 0o700);
        assert_eq!(file_mode, 0o600);

        let _ = fs::remove_dir_all(root);
    }
}

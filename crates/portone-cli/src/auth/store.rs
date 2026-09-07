use std::fmt;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};

use crate::config::OAuthTokens;

pub const KEYRING_SERVICE: &str = "portone-cli";
const KEYRING_TIMEOUT: Duration = Duration::from_secs(30);
const LOCK_TIMEOUT: Duration = Duration::from_secs(30);
const LOCK_POLL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Unavailable(String),
    Timeout,
    Corrupt(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Unavailable(message) => write!(f, "{message}"),
            StoreError::Timeout => write!(
                f,
                "keyring did not respond within {} seconds",
                KEYRING_TIMEOUT.as_secs()
            ),
            StoreError::Corrupt(message) => {
                write!(f, "failed to parse stored tokens: {message}")
            }
        }
    }
}

impl std::error::Error for StoreError {}

pub trait SecretStore {
    fn save(&self, credential_id: &str, tokens: &OAuthTokens) -> Result<(), StoreError>;
    fn load(&self, credential_id: &str) -> Result<Option<OAuthTokens>, StoreError>;
    fn delete(&self, credential_id: &str) -> Result<(), StoreError>;
}

pub struct KeyringStore;

impl SecretStore for KeyringStore {
    fn save(&self, credential_id: &str, tokens: &OAuthTokens) -> Result<(), StoreError> {
        let json =
            serde_json::to_string(tokens).map_err(|err| StoreError::Corrupt(err.to_string()))?;
        let id = credential_id.to_string();
        keyring_op(move || {
            keyring::Entry::new(KEYRING_SERVICE, &id)
                .and_then(|entry| entry.set_password(&json))
                .map_err(|err| err.to_string())
        })
    }

    fn load(&self, credential_id: &str) -> Result<Option<OAuthTokens>, StoreError> {
        let id = credential_id.to_string();
        let json = keyring_op(move || {
            match keyring::Entry::new(KEYRING_SERVICE, &id).and_then(|entry| entry.get_password()) {
                Ok(value) => Ok(Some(value)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(err) => Err(err.to_string()),
            }
        })?;
        json.map(|json| {
            serde_json::from_str(&json).map_err(|err| StoreError::Corrupt(err.to_string()))
        })
        .transpose()
    }

    fn delete(&self, credential_id: &str) -> Result<(), StoreError> {
        let id = credential_id.to_string();
        keyring_op(move || {
            match keyring::Entry::new(KEYRING_SERVICE, &id)
                .and_then(|entry| entry.delete_credential())
            {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(err) => Err(err.to_string()),
            }
        })
    }
}

fn keyring_op<T: Send + 'static>(
    op: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, StoreError> {
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = tx.send(op());
    });
    match rx.recv_timeout(KEYRING_TIMEOUT) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(message)) => Err(StoreError::Unavailable(message)),
        Err(_) => Err(StoreError::Timeout),
    }
}

pub fn new_credential_id() -> anyhow::Result<String> {
    super::oauth::random_base64url(16)
}

pub fn lock_refresh(key: &str) -> anyhow::Result<File> {
    let path = crate::config::paths::config_dir()
        .join("locks")
        .join(format!("{}.lock", sanitize(key)));
    lock_file(
        &path,
        "another portone process is refreshing the token; try again shortly",
    )
}

pub fn lock_config() -> anyhow::Result<File> {
    // Keep the shared config lock separate from credential-specific locks.
    let path = crate::config::paths::config_dir().join("config.lock");
    lock_file(
        &path,
        "another portone process is updating the config file; try again shortly",
    )
}

fn lock_file(path: &Path, busy_message: &str) -> anyhow::Result<File> {
    let dir = path.parent().expect("lock files have a parent directory");
    std::fs::create_dir_all(dir)
        .with_context(|| format!("failed to create lock directory: {}", dir.display()))?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(path)
        .with_context(|| format!("failed to open lock file: {}", path.display()))?;
    let deadline = Instant::now() + LOCK_TIMEOUT;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    bail!("{busy_message}");
                }
                std::thread::sleep(LOCK_POLL);
            }
            Err(TryLockError::Error(err)) => {
                return Err(err)
                    .with_context(|| format!("failed to acquire lock: {}", path.display()));
            }
        }
    }
}

fn sanitize(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod testing {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use super::*;

    #[derive(Clone, Default)]
    pub struct MemoryStore {
        pub entries: Rc<RefCell<HashMap<String, OAuthTokens>>>,
        pub fail_save: Rc<RefCell<Option<StoreError>>>,
        pub fail_load: Rc<RefCell<Option<StoreError>>>,
        pub fail_delete: Rc<RefCell<Option<StoreError>>>,
        pub deleted: Rc<RefCell<Vec<String>>>,
    }

    impl MemoryStore {
        pub fn with(id: &str, tokens: OAuthTokens) -> Self {
            let store = Self::default();
            store.entries.borrow_mut().insert(id.to_string(), tokens);
            store
        }

        pub fn ids(&self) -> Vec<String> {
            let mut ids: Vec<String> = self.entries.borrow().keys().cloned().collect();
            ids.sort();
            ids
        }
    }

    impl SecretStore for MemoryStore {
        fn save(&self, credential_id: &str, tokens: &OAuthTokens) -> Result<(), StoreError> {
            if let Some(err) = self.fail_save.borrow().clone() {
                return Err(err);
            }
            self.entries
                .borrow_mut()
                .insert(credential_id.to_string(), tokens.clone());
            Ok(())
        }

        fn load(&self, credential_id: &str) -> Result<Option<OAuthTokens>, StoreError> {
            if let Some(err) = self.fail_load.borrow().clone() {
                return Err(err);
            }
            Ok(self.entries.borrow().get(credential_id).cloned())
        }

        fn delete(&self, credential_id: &str) -> Result<(), StoreError> {
            if let Some(err) = self.fail_delete.borrow().clone() {
                return Err(err);
            }
            self.deleted.borrow_mut().push(credential_id.to_string());
            self.entries.borrow_mut().remove(credential_id);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::paths::with_env;

    #[test]
    fn keyring_op_times_out_on_slow_worker() {
        let started = Instant::now();
        let result: Result<(), StoreError> = keyring_op_with_timeout(
            || {
                std::thread::sleep(Duration::from_millis(300));
                Ok(())
            },
            Duration::from_millis(50),
        );
        assert_eq!(result, Err(StoreError::Timeout));
        assert!(started.elapsed() < Duration::from_millis(250));
    }

    #[test]
    fn keyring_op_maps_failures_to_unavailable() {
        let result: Result<(), StoreError> =
            keyring_op_with_timeout(|| Err("locked".to_string()), Duration::from_secs(1));
        assert_eq!(result, Err(StoreError::Unavailable("locked".to_string())));
    }

    fn keyring_op_with_timeout<T: Send + 'static>(
        op: impl FnOnce() -> Result<T, String> + Send + 'static,
        timeout: Duration,
    ) -> Result<T, StoreError> {
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let _ = tx.send(op());
        });
        match rx.recv_timeout(timeout) {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(message)) => Err(StoreError::Unavailable(message)),
            Err(_) => Err(StoreError::Timeout),
        }
    }

    #[test]
    fn credential_ids_are_url_safe_and_unique() {
        let a = new_credential_id().unwrap();
        let b = new_credential_id().unwrap();
        assert_ne!(a, b);
        assert_eq!(a.len(), 22);
        assert_eq!(sanitize(&a), a);
        assert_eq!(sanitize("dev profile/1"), "dev_profile_1");
    }

    #[test]
    fn refresh_lock_serializes_holders() {
        let dir = tempfile::tempdir().unwrap();
        with_env(
            &[("PORTONE_CONFIG_DIR", Some(dir.path().to_str().unwrap()))],
            || {
                let first = lock_refresh("cred").unwrap();
                let path = dir.path().join("locks").join("cred.lock");
                assert!(path.exists());
                let second = OpenOptions::new().write(true).open(&path).unwrap();
                assert!(matches!(second.try_lock(), Err(TryLockError::WouldBlock)));
                drop(first);
                assert!(second.try_lock().is_ok());
            },
        );
    }
}

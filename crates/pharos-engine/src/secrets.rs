//! Where secret values live: the OS keyring in the app, anything else in tests. The engine
//! reads a value back only to run an http source that references it (D-031).

pub const KEYRING_SERVICE: &str = "io.pharos.desktop";

/// Errors are short descriptions of the failure and must never contain the value.
pub trait SecretStore: Send {
    /// `None` when there is no entry.
    fn get(&self, account: &str) -> Result<Option<String>, String>;
    fn set(&self, account: &str, value: &str) -> Result<(), String>;
    /// Ok when the entry is already missing.
    fn delete(&self, account: &str) -> Result<(), String>;
}

/// macOS Keychain / Linux Secret Service, service `io.pharos.desktop`.
pub struct KeyringStore;

impl SecretStore for KeyringStore {
    fn get(&self, account: &str) -> Result<Option<String>, String> {
        match keyring::Entry::new(KEYRING_SERVICE, account).and_then(|e| e.get_password()) {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<(), String> {
        keyring::Entry::new(KEYRING_SERVICE, account)
            .and_then(|e| e.set_password(value))
            .map_err(|e| e.to_string())
    }

    fn delete(&self, account: &str) -> Result<(), String> {
        match keyring::Entry::new(KEYRING_SERVICE, account).and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// `<platform>/<SECRET_NAME>`.
pub(crate) fn account(platform: &str, name: &str) -> String {
    format!("{platform}/{name}")
}

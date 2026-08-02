//! Secure secret storage abstraction for LLM credentials.

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Mutex;

/// Stores and retrieves credentials from a platform secret backend.
pub trait SecretStore: Send + Sync {
    fn store(&self, service: &str, account: &str, secret: &str) -> Result<(), String>;
    fn get(&self, service: &str, account: &str) -> Result<String, String>;
    fn delete(&self, service: &str, account: &str) -> Result<(), String>;
}

/// Production secret store backed by macOS Keychain, Windows Credential
/// Manager, or Linux Secret Service via the `keyring` crate.
pub struct KeyringSecretStore;

impl KeyringSecretStore {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KeyringSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeyringSecretStore {
    fn store(&self, service: &str, account: &str, secret: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(service, account).map_err(|e| e.to_string())?;
        entry.set_password(secret).map_err(|e| e.to_string())
    }

    fn get(&self, service: &str, account: &str) -> Result<String, String> {
        let entry = keyring::Entry::new(service, account).map_err(|e| e.to_string())?;
        entry.get_password().map_err(|e| e.to_string())
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(service, account).map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// In-memory secret store for repository tests.
#[cfg(test)]
pub struct InMemorySecretStore {
    secrets: Mutex<HashMap<(String, String), String>>,
    fail_store: Mutex<Option<String>>,
    fail_get: Mutex<Option<String>>,
    fail_delete: Mutex<Option<String>>,
}

#[cfg(test)]
impl InMemorySecretStore {
    pub fn new() -> Self {
        Self {
            secrets: Mutex::new(HashMap::new()),
            fail_store: Mutex::new(None),
            fail_get: Mutex::new(None),
            fail_delete: Mutex::new(None),
        }
    }

    pub fn with_store_failure(message: &str) -> Self {
        let store = Self::new();
        *store.fail_store.lock().expect("fail_store mutex poisoned") = Some(message.to_string());
        store
    }

    pub fn with_get_failure(message: &str) -> Self {
        let store = Self::new();
        *store.fail_get.lock().expect("fail_get mutex poisoned") = Some(message.to_string());
        store
    }
}

#[cfg(test)]
impl Default for InMemorySecretStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl SecretStore for InMemorySecretStore {
    fn store(&self, service: &str, account: &str, secret: &str) -> Result<(), String> {
        if let Some(message) = self
            .fail_store
            .lock()
            .expect("fail_store mutex poisoned")
            .clone()
        {
            return Err(message);
        }

        self.secrets.lock().expect("secrets mutex poisoned").insert(
            (service.to_string(), account.to_string()),
            secret.to_string(),
        );
        Ok(())
    }

    fn get(&self, service: &str, account: &str) -> Result<String, String> {
        if let Some(message) = self
            .fail_get
            .lock()
            .expect("fail_get mutex poisoned")
            .clone()
        {
            return Err(message);
        }

        self.secrets
            .lock()
            .expect("secrets mutex poisoned")
            .get(&(service.to_string(), account.to_string()))
            .cloned()
            .ok_or_else(|| "credential not found".to_string())
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), String> {
        if let Some(message) = self
            .fail_delete
            .lock()
            .expect("fail_delete mutex poisoned")
            .clone()
        {
            return Err(message);
        }

        self.secrets
            .lock()
            .expect("secrets mutex poisoned")
            .remove(&(service.to_string(), account.to_string()));
        Ok(())
    }
}

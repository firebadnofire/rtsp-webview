use serde::{Deserialize, Serialize};

pub const DEFAULT_SERVICE_NAME: &str = "com.rtspviewer.app";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretPayload {
    pub username: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("failed to access secret backend: {0}")]
    Backend(String),
    #[error("failed to serialize secret payload: {0}")]
    Serialization(String),
    #[error("failed to deserialize secret payload: {0}")]
    Deserialization(String),
}

pub trait SecretStore: Send + Sync {
    fn set(&self, key: &str, payload: SecretPayload) -> Result<(), SecretError>;
    fn get(&self, key: &str) -> Result<Option<SecretPayload>, SecretError>;
    fn delete(&self, key: &str) -> Result<(), SecretError>;
}

#[derive(Debug, Clone)]
pub struct KeyringSecretStore {
    service_name: String,
}

impl Default for KeyringSecretStore {
    fn default() -> Self {
        Self::new(DEFAULT_SERVICE_NAME)
    }
}

impl KeyringSecretStore {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(&self.service_name, key)
            .map_err(|error| SecretError::Backend(error.to_string()))
    }
}

impl SecretStore for KeyringSecretStore {
    fn set(&self, key: &str, payload: SecretPayload) -> Result<(), SecretError> {
        let entry = self.entry(key)?;
        let serialized = serde_json::to_string(&payload)
            .map_err(|error| SecretError::Serialization(error.to_string()))?;
        entry
            .set_password(&serialized)
            .map_err(|error| SecretError::Backend(error.to_string()))
    }

    fn get(&self, key: &str) -> Result<Option<SecretPayload>, SecretError> {
        let entry = self.entry(key)?;
        let value = match entry.get_password() {
            Ok(value) => value,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(error) => return Err(SecretError::Backend(error.to_string())),
        };

        let payload = serde_json::from_str::<SecretPayload>(&value)
            .map_err(|error| SecretError::Deserialization(error.to_string()))?;
        Ok(Some(payload))
    }

    fn delete(&self, key: &str) -> Result<(), SecretError> {
        let entry = self.entry(key)?;
        match entry.delete_credential() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(SecretError::Backend(error.to_string())),
        }
    }
}

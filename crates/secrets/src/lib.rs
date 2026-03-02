#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPayload {
    pub username: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("secret store is not implemented")]
    NotImplemented,
}

pub trait SecretStore {
    fn set(&self, _key: &str, _payload: SecretPayload) -> Result<(), SecretError>;
    fn get(&self, _key: &str) -> Result<Option<SecretPayload>, SecretError>;
}

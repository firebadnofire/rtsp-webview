use rtsp_core::ValidationError;
use serde::Serialize;

#[derive(Debug, thiserror::Error, Serialize, Clone)]
#[error("{message}")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl CommandError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::new("E_CONFIG_INVALID", message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new("E_IO", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("E_INTERNAL", message)
    }

    pub fn decode(message: impl Into<String>) -> Self {
        Self::new("E_DECODE", message)
    }
}

impl From<ValidationError> for CommandError {
    fn from(value: ValidationError) -> Self {
        Self::new(value.code(), value.user_message())
    }
}

impl From<std::io::Error> for CommandError {
    fn from(value: std::io::Error) -> Self {
        Self::io(value.to_string())
    }
}

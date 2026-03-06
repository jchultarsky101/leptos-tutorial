use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in the UI when communicating with the API.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum UiError {
    #[error("network error: {0}")]
    Network(String),
    #[error("server error {status}: {message}")]
    Api { status: u16, message: String },
    #[error("failed to parse response: {0}")]
    Parse(String),
    #[error("file too large to preview ({0} bytes; limit is 1 MiB)")]
    FileTooLarge(u64),
}

impl UiError {
    /// Build a `UiError::Api` from an HTTP status and a plain message string.
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
        }
    }
}

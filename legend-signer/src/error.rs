use thiserror::Error;

#[derive(Error, Debug)]
pub enum SignerError {
    #[error("Secure Enclave error: {0}")]
    SecureEnclave(String),

    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Turnkey API error: {0}")]
    Turnkey(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("Time error: {0}")]
    Time(#[from] std::time::SystemTimeError),
}

pub type Result<T> = std::result::Result<T, SignerError>;

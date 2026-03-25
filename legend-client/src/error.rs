use thiserror::Error;

#[derive(Error, Debug)]
pub enum LegendPrimeError {
    #[error("API error ({status}): [{code}] {message}")]
    Api {
        code: String,
        message: String,
        status: u16,
    },

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Deserialization error: {0}")]
    Deserialize(serde_json::Error),
}

pub type Result<T> = std::result::Result<T, LegendPrimeError>;

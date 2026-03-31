use thiserror::Error;

#[derive(Error, Debug)]
pub enum LegendPrimeError {
    #[error("{}", format_api_error(.status, .code, .message, .details))]
    Api {
        code: String,
        message: String,
        status: u16,
        details: Option<serde_json::Value>,
    },

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Deserialization error: {0}")]
    Deserialize(serde_json::Error),
}

fn format_api_error(
    status: &u16,
    code: &str,
    message: &str,
    details: &Option<serde_json::Value>,
) -> String {
    match details {
        Some(d) => format!("API error ({status}): [{code}] {message}\nDetails: {d}"),
        None => format!("API error ({status}): [{code}] {message}"),
    }
}

pub type Result<T> = std::result::Result<T, LegendPrimeError>;

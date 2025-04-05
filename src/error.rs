use thiserror::Error;

/// Error types for Telegrama operations
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error (missing or invalid settings)
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// API error from Telegram
    #[error("Telegram API error: {0}")]
    Api(String),

    /// Error related to message formatting
    #[error("Formatting error: {0}")]
    Formatting(String),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create a new configuration error
    pub fn configuration<S: AsRef<str>>(message: S) -> Self {
        Error::Configuration(message.as_ref().to_string())
    }

    /// Create a new API error
    pub fn api<S: AsRef<str>>(message: S) -> Self {
        Error::Api(message.as_ref().to_string())
    }

    /// Create a new formatting error
    pub fn formatting<S: AsRef<str>>(message: S) -> Self {
        Error::Formatting(message.as_ref().to_string())
    }

    /// Create a new generic error
    pub fn other<S: AsRef<str>>(message: S) -> Self {
        Error::Other(message.as_ref().to_string())
    }
}

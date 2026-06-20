use thiserror::Error;

/// Represents all possible errors that can occur when querying a Minecraft server.
#[derive(Error, Debug)]
pub enum McStatusError {
    /// An I/O error occurred during network communication.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse JSON from the server response.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// The server responded with data that violates the expected protocol.
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// DNS resolution failed.
    #[cfg(feature = "dns")]
    #[error("DNS resolution failed: {0}")]
    Dns(String),

    /// The connection timed out before receiving a response.
    #[error("Connection timed out")]
    Timeout,

    /// The server's response was malformed or missing required fields.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// The server did not respond with any information.
    #[error("Server did not respond with any information")]
    NoResponse,

    /// The provided address is invalid.
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// A UTF-8 decoding error occurred.
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Failed to parse an integer from a string.
    #[error("Integer parse error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    /// A catch-all for other error conditions.
    #[error("{0}")]
    Other(String),
}

/// Convenience result type alias.
pub type Result<T> = std::result::Result<T, McStatusError>;

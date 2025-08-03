use std::fmt;

/// Custom error types for HTTP operations
///
/// Represents various failure modes that can occur during HTTP requests,
/// from network connectivity issues to parsing problems.
#[derive(Debug, Clone)]
pub enum HttpError {
    /// Network-related errors (connection failed, timeout, etc.)
    Network(String),
    /// HTTP parsing errors (malformed response, invalid headers, etc.)
    Parse(String),
    /// Unsupported HTTP method
    UnsupportedMethod(String),
    /// Other errors
    Other(String),
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpError::Network(msg) => write!(f, "Network error: {msg}"),
            HttpError::Parse(msg) => write!(f, "Parse error: {msg}"),
            HttpError::UnsupportedMethod(method) => {
                write!(f, "Unsupported HTTP method: {method}")
            }
            HttpError::Other(msg) => write!(f, "Error: {msg}"),
        }
    }
}

impl std::error::Error for HttpError {}


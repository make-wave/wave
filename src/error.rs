//! Error handling for the wave HTTP client
//!
//! This module provides a comprehensive error type hierarchy for all failure modes
//! in the wave application. All errors implement helpful `Display` messages and many
//! provide actionable suggestions to help users resolve issues.
//!
//! The main [`WaveError`] enum unifies all possible errors, while specific error
//! types like [`CollectionError`] and [`CliError`] provide detailed context for
//! different failure domains.

use std::fmt;
use std::io;

use crate::http_client::HttpError;

/// Central error type for the wave application
///
/// Unifies all possible error conditions that can occur during wave execution.
/// Each variant wraps a more specific error type that provides detailed context
/// and user-friendly error messages.
///
/// # Examples
/// ```
/// use wave::error::WaveError;
///
/// // Errors automatically convert from specific types
/// let result: Result<(), WaveError> = Err(wave::error::invalid_url("example.com"));
///
/// // All errors provide helpful messages
/// if let Err(e) = result {
///     println!("Error: {}", e);
///     if let Some(suggestion) = e.suggestion() {
///         println!("Suggestion: {}", suggestion);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum WaveError {
    /// HTTP-related errors from network requests
    Http(HttpError),
    /// Collection/YAML file errors  
    Collection(CollectionError),
    /// CLI argument parsing and validation errors
    Cli(CliError),
    /// I/O errors from file system operations
    Io(String),
    /// JSON/YAML parsing errors
    Parse(ParseError),
    /// Configuration file errors
    Config(ConfigError),
    /// Runtime and system errors
    Runtime(String),
}

/// Collection and YAML related errors
///
/// Covers all error conditions related to loading, parsing, and using
/// collection files and their associated operations.
#[derive(Debug, Clone)]
pub enum CollectionError {
    /// Collection file not found at specified path
    FileNotFound(String),
    /// Invalid YAML syntax or structure in collection file
    InvalidYaml(String),
    /// Requested request name not found in collection
    RequestNotFound { collection: String, request: String },
    /// Variable resolution failed during collection processing
    VariableResolution(String),
    /// Collection directory (.wave/) not found
    DirectoryNotFound(String),
}

/// CLI argument parsing and validation errors
///
/// Covers all error conditions related to command-line argument processing,
/// URL validation, and parameter format checking.
#[derive(Debug, Clone)]
pub enum CliError {
    /// URL format is invalid or malformed
    InvalidUrl(String),
    /// Required command-line arguments are missing
    MissingArguments(String),
    /// Header parameter not in 'key:value' format
    InvalidHeaderFormat(String),
    /// Body parameter not in 'key=value' format
    InvalidBodyFormat(String),
    /// HTTP method is not supported
    UnsupportedMethod(String),
}

/// Parsing related errors
///
/// Covers errors that occur when parsing various data formats used
/// throughout the application.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// JSON parsing or serialization error
    Json(String),
    /// YAML parsing or serialization error
    Yaml(String),
    /// HTTP header parsing error
    Header(String),
    /// URL parsing error
    Url(String),
}

/// Configuration related errors
///
/// Covers errors related to application configuration files and settings.
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// Configuration file is malformed or invalid
    InvalidConfig(String),
    /// Required configuration is missing
    MissingConfig(String),
}

impl fmt::Display for WaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WaveError::Http(err) => write!(f, "{err}"),
            WaveError::Collection(err) => write!(f, "{err}"),
            WaveError::Cli(err) => write!(f, "{err}"),
            WaveError::Io(msg) => write!(f, "I/O error: {msg}"),
            WaveError::Parse(err) => write!(f, "{err}"),
            WaveError::Config(err) => write!(f, "{err}"),
            WaveError::Runtime(msg) => write!(f, "Runtime error: {msg}"),
        }
    }
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CollectionError::FileNotFound(path) => {
                write!(f, "Collection file not found: '{path}'. Try running 'wave init' to create a collection.")
            }
            CollectionError::InvalidYaml(msg) => {
                write!(f, "Invalid YAML in collection file: {msg}")
            }
            CollectionError::RequestNotFound {
                collection,
                request,
            } => {
                write!(f, "Request '{request}' not found in collection '{collection}'. Check the collection YAML file to see available requests.")
            }
            CollectionError::VariableResolution(msg) => {
                write!(f, "Failed to resolve variables: {msg}")
            }
            CollectionError::DirectoryNotFound(path) => {
                write!(f, "Collection directory not found: '{path}'. Try running 'wave init' to create a collection.")
            }
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::InvalidUrl(url) => {
                write!(
                    f,
                    "Invalid URL '{url}'. URLs must include protocol (http:// or https://)"
                )
            }
            CliError::MissingArguments(msg) => {
                write!(f, "Missing required arguments: {msg}")
            }
            CliError::InvalidHeaderFormat(header) => {
                write!(
                    f,
                    "Invalid header format '{header}'. Headers must be in 'key:value' format"
                )
            }
            CliError::InvalidBodyFormat(body) => {
                write!(
                    f,
                    "Invalid body format '{body}'. Body data must be in 'key=value' format"
                )
            }
            CliError::UnsupportedMethod(method) => {
                write!(f, "Unsupported HTTP method: '{method}'. Supported methods: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS")
            }
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Json(msg) => {
                write!(f, "JSON parsing error: {msg}")
            }
            ParseError::Yaml(msg) => {
                write!(f, "YAML parsing error: {msg}")
            }
            ParseError::Header(msg) => {
                write!(f, "Header parsing error: {msg}")
            }
            ParseError::Url(msg) => {
                write!(f, "URL parsing error: {msg}")
            }
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidConfig(msg) => {
                write!(f, "Invalid configuration: {msg}")
            }
            ConfigError::MissingConfig(msg) => {
                write!(f, "Missing configuration: {msg}")
            }
        }
    }
}

impl std::error::Error for WaveError {}
impl std::error::Error for CollectionError {}
impl std::error::Error for CliError {}
impl std::error::Error for ParseError {}
impl std::error::Error for ConfigError {}

// Conversion implementations for easier error handling
impl From<HttpError> for WaveError {
    fn from(err: HttpError) -> Self {
        WaveError::Http(err)
    }
}

impl From<CollectionError> for WaveError {
    fn from(err: CollectionError) -> Self {
        WaveError::Collection(err)
    }
}

impl From<CliError> for WaveError {
    fn from(err: CliError) -> Self {
        WaveError::Cli(err)
    }
}

impl From<ParseError> for WaveError {
    fn from(err: ParseError) -> Self {
        WaveError::Parse(err)
    }
}

impl From<ConfigError> for WaveError {
    fn from(err: ConfigError) -> Self {
        WaveError::Config(err)
    }
}

impl From<io::Error> for WaveError {
    fn from(err: io::Error) -> Self {
        WaveError::Io(err.to_string())
    }
}

impl From<serde_yaml::Error> for WaveError {
    fn from(err: serde_yaml::Error) -> Self {
        WaveError::Parse(ParseError::Yaml(err.to_string()))
    }
}

impl From<serde_json::Error> for WaveError {
    fn from(err: serde_json::Error) -> Self {
        WaveError::Parse(ParseError::Json(err.to_string()))
    }
}

impl WaveError {
    /// Provides actionable suggestions for resolving errors
    ///
    /// Returns helpful guidance for common error conditions that users
    /// can act upon to resolve the issue.
    ///
    /// # Returns
    /// `Some(suggestion)` for errors with actionable solutions, `None` for
    /// errors that don't have clear user remediation steps.
    ///
    /// # Examples
    /// ```
    /// use wave::error::{WaveError, invalid_url};
    ///
    /// let err = invalid_url("example.com");
    /// if let Some(suggestion) = err.suggestion() {
    ///     println!("Try: {}", suggestion);
    ///     // Prints: "Try: Example: wave get https://api.example.com/users"
    /// }
    /// ```
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            WaveError::Collection(CollectionError::FileNotFound(_)) => {
                Some("Make sure the file exists in the .wave directory")
            }
            WaveError::Collection(CollectionError::DirectoryNotFound(_)) => {
                Some("No .wave directory found for collections")
            }
            WaveError::Collection(CollectionError::RequestNotFound { .. }) => {
                Some("Check the collection YAML file to see all available requests")
            }
            WaveError::Cli(CliError::InvalidUrl(_)) => {
                Some("Example: wave get https://api.example.com/users")
            }
            WaveError::Cli(CliError::InvalidHeaderFormat(_)) => {
                Some("Example: Authorization:Bearer123 Content-Type:application/json")
            }
            WaveError::Cli(CliError::InvalidBodyFormat(_)) => {
                Some("Example: name=john age=30 active=true")
            }
            _ => None,
        }
    }
}

/// Creates a collection file not found error
///
/// # Arguments
/// * `path` - The path to the missing collection file
///
/// # Examples
/// ```
/// use wave::error::collection_file_not_found;
///
/// let err = collection_file_not_found(".wave/api.yml");
/// assert!(err.to_string().contains("Collection file not found"));
/// ```
pub fn collection_file_not_found(path: &str) -> WaveError {
    WaveError::Collection(CollectionError::FileNotFound(path.to_string()))
}

/// Creates an invalid URL error
///
/// # Arguments  
/// * `url` - The invalid URL that was provided
///
/// # Examples
/// ```
/// use wave::error::invalid_url;
///
/// let err = invalid_url("not-a-url");
/// assert!(err.to_string().contains("Invalid URL"));
/// assert!(err.suggestion().is_some());
/// ```
pub fn invalid_url(url: &str) -> WaveError {
    WaveError::Cli(CliError::InvalidUrl(url.to_string()))
}

/// Creates a runtime error
///
/// # Arguments
/// * `msg` - Description of the runtime error
pub fn runtime_error(msg: &str) -> WaveError {
    WaveError::Runtime(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CollectionError::FileNotFound("test.yaml".to_string());
        assert!(err.to_string().contains("Collection file not found"));
        assert!(err.to_string().contains("wave init"));
    }

    #[test]
    fn test_cli_error_display() {
        let err = CliError::InvalidUrl("example.com".to_string());
        assert!(err.to_string().contains("Invalid URL"));
        assert!(err.to_string().contains("http://"));
    }

    #[test]
    fn test_wave_error_suggestion() {
        let err = WaveError::Collection(CollectionError::FileNotFound("test.yaml".to_string()));
        assert!(err.suggestion().is_some());
        assert!(err.suggestion().unwrap().contains(".wave directory"));
    }

    #[test]
    fn test_error_conversions() {
        let http_err = HttpError::Network("connection failed".to_string());
        let wave_err: WaveError = http_err.into();
        matches!(wave_err, WaveError::Http(_));
    }

    #[test]
    fn test_error_chain_conversion() {
        // Test that different error types convert properly
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let wave_err: WaveError = io_err.into();
        assert!(wave_err.to_string().contains("I/O error"));

        // Test YAML error by parsing invalid YAML
        let yaml_result: Result<serde_yaml::Value, serde_yaml::Error> =
            serde_yaml::from_str("invalid: yaml: content:");
        if let Err(yaml_err) = yaml_result {
            let wave_err: WaveError = yaml_err.into();
            assert!(wave_err.to_string().contains("YAML parsing error"));
        }
    }

    #[test]
    fn test_all_error_variants_display() {
        let errors = vec![
            WaveError::Runtime("test runtime error".to_string()),
            WaveError::Io("test io error".to_string()),
            WaveError::Collection(CollectionError::DirectoryNotFound("test-dir".to_string())),
            WaveError::Cli(CliError::UnsupportedMethod("INVALID".to_string())),
            WaveError::Parse(ParseError::Url("invalid url".to_string())),
            WaveError::Config(ConfigError::InvalidConfig("bad config".to_string())),
        ];

        for err in errors {
            // All errors should display without panicking
            let _ = err.to_string();
            let _ = format!("{err:?}");
        }
    }

    #[test]
    fn test_suggestion_coverage() {
        let suggestions = vec![
            (
                WaveError::Collection(CollectionError::FileNotFound("test.yaml".to_string())),
                true,
            ),
            (
                WaveError::Collection(CollectionError::DirectoryNotFound("test".to_string())),
                true,
            ),
            (
                WaveError::Collection(CollectionError::RequestNotFound {
                    collection: "test".to_string(),
                    request: "req".to_string(),
                }),
                true,
            ),
            (
                WaveError::Cli(CliError::InvalidUrl("bad-url".to_string())),
                true,
            ),
            (
                WaveError::Cli(CliError::InvalidHeaderFormat("bad:header".to_string())),
                true,
            ),
            (
                WaveError::Cli(CliError::InvalidBodyFormat("bad=body".to_string())),
                true,
            ),
            (WaveError::Runtime("runtime error".to_string()), false),
        ];

        for (err, should_have_suggestion) in suggestions {
            if should_have_suggestion {
                assert!(
                    err.suggestion().is_some(),
                    "Error should have suggestion: {err}"
                );
            } else {
                assert!(
                    err.suggestion().is_none(),
                    "Error should not have suggestion: {err}"
                );
            }
        }
    }

    #[test]
    fn test_request_not_found_error_message() {
        let err = CollectionError::RequestNotFound {
            collection: "api".to_string(),
            request: "missing_request".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Request 'missing_request' not found in collection 'api'"));
        assert!(msg.contains("Check the collection YAML file"));
        assert!(!msg.contains("wave list")); // Ensure old message is gone
    }
}

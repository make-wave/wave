use std::fmt;
use std::io;

use crate::http_client::HttpError;

/// Central error type for the wave application
#[derive(Debug, Clone)]
pub enum WaveError {
    /// HTTP-related errors
    Http(HttpError),
    /// Collection/YAML errors
    Collection(CollectionError),
    /// CLI argument parsing errors
    Cli(CliError),
    /// I/O errors (file operations)
    Io(String),
    /// JSON/YAML parsing errors
    Parse(ParseError),
    /// Configuration errors
    Config(ConfigError),
    /// Runtime/system errors
    Runtime(String),
}

/// Collection and YAML related errors
#[derive(Debug, Clone)]
pub enum CollectionError {
    /// Collection file not found
    FileNotFound(String),
    /// Invalid YAML content
    InvalidYaml(String),
    /// Requested request not found in collection
    RequestNotFound { collection: String, request: String },
    /// Variable resolution failed
    VariableResolution(String),
    /// Collection directory not found
    DirectoryNotFound(String),
}

/// CLI argument parsing and validation errors
#[derive(Debug, Clone)]
pub enum CliError {
    /// Invalid URL format
    InvalidUrl(String),
    /// Missing required arguments
    MissingArguments(String),
    /// Invalid header format
    InvalidHeaderFormat(String),
    /// Invalid body format
    InvalidBodyFormat(String),
    /// Unsupported method
    UnsupportedMethod(String),
}

/// Parsing related errors
#[derive(Debug, Clone)]
pub enum ParseError {
    /// JSON parsing error
    Json(String),
    /// YAML parsing error
    Yaml(String),
    /// Header parsing error
    Header(String),
    /// URL parsing error
    Url(String),
}

/// Configuration related errors
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// Invalid configuration file
    InvalidConfig(String),
    /// Missing configuration
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
            CollectionError::RequestNotFound { collection, request } => {
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
                write!(f, "Invalid URL '{url}'. URLs must include protocol (http:// or https://)")
            }
            CliError::MissingArguments(msg) => {
                write!(f, "Missing required arguments: {msg}")
            }
            CliError::InvalidHeaderFormat(header) => {
                write!(f, "Invalid header format '{header}'. Headers must be in 'key:value' format")
            }
            CliError::InvalidBodyFormat(body) => {
                write!(f, "Invalid body format '{body}'. Body data must be in 'key=value' format")
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
    /// Provides a helpful suggestion for how to fix the error
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            WaveError::Collection(CollectionError::FileNotFound(_)) => {
                Some("Run 'wave init' to create a new collection in the current directory")
            }
            WaveError::Collection(CollectionError::DirectoryNotFound(_)) => {
                Some("Run 'wave init' to create a new collection in the current directory")
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

/// Helper function to create collection file not found error
pub fn collection_file_not_found(path: &str) -> WaveError {
    WaveError::Collection(CollectionError::FileNotFound(path.to_string()))
}

/// Helper function to create invalid URL error
pub fn invalid_url(url: &str) -> WaveError {
    WaveError::Cli(CliError::InvalidUrl(url.to_string()))
}

/// Helper function to create runtime error
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
        assert!(err.suggestion().unwrap().contains("wave init"));
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
        let yaml_result: Result<serde_yaml::Value, serde_yaml::Error> = serde_yaml::from_str("invalid: yaml: content:");
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
            (WaveError::Collection(CollectionError::FileNotFound("test.yaml".to_string())), true),
            (WaveError::Collection(CollectionError::DirectoryNotFound("test".to_string())), true),
            (WaveError::Collection(CollectionError::RequestNotFound { collection: "test".to_string(), request: "req".to_string() }), true),
            (WaveError::Cli(CliError::InvalidUrl("bad-url".to_string())), true),
            (WaveError::Cli(CliError::InvalidHeaderFormat("bad:header".to_string())), true),
            (WaveError::Cli(CliError::InvalidBodyFormat("bad=body".to_string())), true),
            (WaveError::Runtime("runtime error".to_string()), false),
        ];

        for (err, should_have_suggestion) in suggestions {
            if should_have_suggestion {
                assert!(err.suggestion().is_some(), "Error should have suggestion: {err}");
            } else {
                assert!(err.suggestion().is_none(), "Error should not have suggestion: {err}");
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
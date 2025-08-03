use crate::http::error::HttpError;
use crate::KeyValuePairs;
use ::http::{HeaderMap, Method};

/// Represents different types of request bodies with automatic serialization
///
/// Provides type-safe handling of various request body formats with automatic
/// Content-Type header management and proper encoding for each format.
///
/// # Examples
///
/// ```
/// use wave::http::RequestBody;
/// use std::collections::HashMap;
///
/// // JSON body
/// let mut data = HashMap::new();
/// data.insert("name", "Alice");
/// let json_body = RequestBody::json(&data)?;
///
/// // Form data
/// let form_data = vec![("username".to_string(), "alice".to_string())];
/// let form_body = RequestBody::form(form_data);
///
/// // Plain text
/// let text_body = RequestBody::text("Hello, world!".to_string());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone)]
pub enum RequestBody {
    /// JSON object body - automatically sets Content-Type to application/json
    Json(serde_json::Value),
    /// Form-encoded body - automatically sets Content-Type to application/x-www-form-urlencoded
    Form(KeyValuePairs),
    /// Plain text body - automatically sets Content-Type to text/plain
    Text(String),
    /// Binary data body - automatically sets Content-Type to application/octet-stream
    Bytes(Vec<u8>),
}

impl RequestBody {
    /// Create a JSON body from any serializable type
    ///
    /// Converts any type implementing `serde::Serialize` into a JSON request body.
    /// This is the recommended way to send structured data to APIs.
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::http::RequestBody;
    /// use std::collections::HashMap;
    ///
    /// let mut data = HashMap::new();
    /// data.insert("username", "alice");
    /// data.insert("email", "alice@example.com");
    ///
    /// let body = RequestBody::json(&data)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn json<T: serde::Serialize>(data: &T) -> Result<Self, HttpError> {
        let value = serde_json::to_value(data)
            .map_err(|e| HttpError::Parse(format!("Failed to serialize JSON: {e}")))?;
        Ok(RequestBody::Json(value))
    }

    /// Create a form-encoded body
    ///
    /// Creates a URL-encoded form body suitable for HTML form submissions.
    /// Automatically handles URL encoding of keys and values.
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::http::RequestBody;
    ///
    /// let form_data = vec![
    ///     ("username".to_string(), "alice".to_string()),
    ///     ("password".to_string(), "secret123".to_string()),
    /// ];
    /// let body = RequestBody::form(form_data);
    /// ```
    pub fn form(data: KeyValuePairs) -> Self {
        RequestBody::Form(data)
    }

    /// Create a plain text body
    ///
    /// For sending plain text content such as logs, notes, or simple data.
    pub fn text(data: String) -> Self {
        RequestBody::Text(data)
    }

    /// Create a binary body
    ///
    /// For sending binary data such as images, files, or other non-text content.
    pub fn bytes(data: Vec<u8>) -> Self {
        RequestBody::Bytes(data)
    }

    /// Serialize the body to a string and set appropriate Content-Type header
    ///
    /// Converts the body to its wire format and automatically sets the correct
    /// Content-Type header based on the body type. This method is used internally
    /// when building HTTP requests.
    pub fn serialize(&self, headers: &mut HeaderMap) -> String {
        match self {
            RequestBody::Json(value) => {
                Self::ensure_content_type(headers, "application/json");
                serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
            }
            RequestBody::Form(data) => {
                Self::ensure_content_type(headers, "application/x-www-form-urlencoded");
                data.iter()
                    .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                    .collect::<Vec<_>>()
                    .join("&")
            }
            RequestBody::Text(text) => {
                Self::ensure_content_type(headers, "text/plain");
                text.clone()
            }
            RequestBody::Bytes(bytes) => {
                Self::ensure_content_type(headers, "application/octet-stream");
                String::from_utf8_lossy(bytes).to_string()
            }
        }
    }

    /// Ensure Content-Type header is set if not already present
    pub(crate) fn ensure_content_type(headers: &mut HeaderMap, content_type: &str) {
        if !headers.contains_key("content-type") {
            headers.insert("content-type", content_type.parse().unwrap());
        }
    }
}

/// Builder for constructing HTTP requests with a fluent API
///
/// Provides a convenient way to build complex HTTP requests step by step.
/// The builder automatically handles header management and body serialization.
///
/// # Examples
///
/// ```
/// use wave::http::{RequestBuilder, RequestBody};
/// use http::Method;
/// use std::collections::HashMap;
///
/// let mut user_data = HashMap::new();
/// user_data.insert("name", "Alice");
/// user_data.insert("email", "alice@example.com");
///
/// let request = RequestBuilder::new("https://api.example.com/users", Method::POST)
///     .header("Authorization", "Bearer token123")
///     .header("Accept", "application/json")
///     .body(RequestBody::json(&user_data)?)
///     .build();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct RequestBuilder {
    url: String,
    method: Method,
    headers: HeaderMap,
    body: Option<RequestBody>,
}

impl RequestBuilder {
    /// Create a new request builder
    ///
    /// Initializes a builder with the specified URL and HTTP method.
    /// Additional configuration can be added using the fluent API methods.
    pub fn new(url: impl Into<String>, method: Method) -> Self {
        Self {
            url: url.into(),
            method,
            headers: HeaderMap::new(),
            body: None,
        }
    }

    /// Add a header to the request
    ///
    /// Sets a single header key-value pair. If the header already exists,
    /// it will be replaced with the new value.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key_str = key.into();
        let value_str = value.into();
        if let (Ok(header_name), Ok(header_value)) = (
            key_str.parse::<::http::HeaderName>(),
            value_str.parse::<::http::HeaderValue>(),
        ) {
            self.headers.insert(header_name, header_value);
        }
        self
    }

    /// Add multiple headers to the request
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Add multiple headers from Vec (convenience method for backward compatibility)
    pub fn headers_from_vec(mut self, headers: KeyValuePairs) -> Self {
        for (key, value) in headers {
            if let (Ok(header_name), Ok(header_value)) = (
                key.parse::<::http::HeaderName>(),
                value.parse::<::http::HeaderValue>(),
            ) {
                self.headers.insert(header_name, header_value);
            }
        }
        self
    }

    /// Set the request body
    ///
    /// Sets the request body using a `RequestBody` instance. Use `RequestBody` static methods
    /// to create the appropriate body type (JSON, form, text, or binary).
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::http::{RequestBuilder, RequestBody};
    /// use http::Method;
    /// use std::collections::HashMap;
    ///
    /// // JSON body
    /// let mut data = HashMap::new();
    /// data.insert("name", "Alice");
    /// let request = RequestBuilder::new("https://api.example.com/users", Method::POST)
    ///     .body(RequestBody::json(&data)?)
    ///     .build();
    ///
    /// // Form body
    /// let form_data = vec![("username".to_string(), "alice".to_string())];
    /// let request2 = RequestBuilder::new("https://api.example.com/login", Method::POST)
    ///     .body(RequestBody::form(form_data))
    ///     .build();
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn body(mut self, body: RequestBody) -> Self {
        self.body = Some(body);
        self
    }

    /// Build the final HttpRequest
    ///
    /// Consumes the builder and produces an `HttpRequest` ready to be sent.
    /// This method handles final serialization of the body and header setup.
    pub fn build(self) -> HttpRequest {
        let mut headers = self.headers;
        let body = self.body.map(|b| b.serialize(&mut headers));

        HttpRequest {
            url: self.url,
            method: self.method,
            body,
            headers,
        }
    }
}

/// Represents an HTTP request with URL, method, body, and headers
///
/// The core request structure used throughout the HTTP client. Can be constructed
/// directly using `HttpRequest::new()` for simple requests, or via the builder pattern
/// using `HttpRequest::builder()` for more complex requests.
///
/// # Examples
///
/// ```
/// use wave::http::{HttpRequest, RequestBody};
/// use http::{HeaderMap, Method};
///
/// // Simple request
/// let mut headers = HeaderMap::new();
/// headers.insert("authorization", "Bearer token123".parse().unwrap());
/// let simple_request = HttpRequest::new(
///     "https://api.example.com/users",
///     Method::GET,
///     None,
///     headers
/// );
///
/// // Complex request using builder
/// let complex_request = HttpRequest::builder("https://api.example.com/users", Method::POST)
///     .header("Authorization", "Bearer token123")
///     .body(RequestBody::json(&serde_json::json!({"name": "Alice"}))?)
///     .build();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct HttpRequest {
    /// Target URL for the request
    pub url: String,
    /// HTTP method to use
    pub method: Method,
    /// Optional request body
    pub body: Option<String>,
    /// HTTP headers to send
    pub headers: HeaderMap,
}

impl HttpRequest {
    /// Constructs a new HttpRequest
    ///
    /// Creates a basic HTTP request with the specified URL, method, body, and headers.
    /// For more complex requests, use `HttpRequest::builder()` instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::http::HttpRequest;
    /// use http::{HeaderMap, Method};
    ///
    /// let mut headers = HeaderMap::new();
    /// headers.insert("content-type", "application/json".parse().unwrap());
    ///
    /// let request = HttpRequest::new(
    ///     "https://api.example.com/users",
    ///     Method::POST,
    ///     Some(r#"{"name": "Alice"}"#.to_string()),
    ///     headers
    /// );
    /// ```
    pub fn new(url: &str, method: Method, body: Option<String>, headers: HeaderMap) -> Self {
        Self {
            url: url.to_string(),
            method,
            body,
            headers,
        }
    }

    /// Create a request builder for complex requests
    ///
    /// Returns a `RequestBuilder` for constructing requests with the fluent API.
    /// This is the recommended approach for requests with multiple headers or complex bodies.
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::http::{HttpRequest, RequestBody};
    /// use http::Method;
    ///
    /// let request = HttpRequest::builder("https://api.example.com/users", Method::POST)
    ///     .header("Authorization", "Bearer token123")
    ///     .header("Content-Type", "application/json")
    ///     .body(RequestBody::text(r#"{"name": "Alice"}"#.to_string()))
    ///     .build();
    /// ```
    pub fn builder(url: impl Into<String>, method: Method) -> RequestBuilder {
        RequestBuilder::new(url, method)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_request_construction() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test", "1".parse().unwrap());

        let req = HttpRequest::new(
            "http://example.com",
            Method::POST,
            Some("body".to_string()),
            headers.clone(),
        );
        assert_eq!(req.url, "http://example.com");
        assert_eq!(req.method, Method::POST);
        assert_eq!(req.body, Some("body".to_string()));
        assert_eq!(req.headers, headers);
    }

    #[test]
    fn test_request_body_form_encoding() {
        let mut headers = HeaderMap::new();
        let data = vec![
            ("foo".to_string(), "bar baz".to_string()),
            ("qux".to_string(), "1&2".to_string()),
        ];
        let body = RequestBody::form(data);
        let encoded = body.serialize(&mut headers);
        assert_eq!(encoded, "foo=bar%20baz&qux=1%262");
        assert!(headers.contains_key("content-type"));
        assert_eq!(
            headers.get("content-type").unwrap(),
            "application/x-www-form-urlencoded"
        );
    }

    #[test]
    fn test_request_body_json_encoding() {
        let mut headers = HeaderMap::new();
        let data: std::collections::HashMap<String, String> = vec![
            ("foo".to_string(), "bar".to_string()),
            ("baz".to_string(), "qux".to_string()),
        ]
        .into_iter()
        .collect();
        let body = RequestBody::json(&data).unwrap();
        let encoded = body.serialize(&mut headers);
        let encoded_json: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        let expected_json: serde_json::Value = serde_json::json!({"foo": "bar", "baz": "qux"});
        assert_eq!(encoded_json, expected_json);
        assert!(headers.contains_key("content-type"));
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
    }

    #[test]
    fn test_request_body_text() {
        let body = RequestBody::text("Hello, World!".to_string());
        let mut headers = HeaderMap::new();
        let serialized = body.serialize(&mut headers);

        assert!(headers.contains_key("content-type"));
        assert_eq!(headers.get("content-type").unwrap(), "text/plain");
        assert_eq!(serialized, "Hello, World!");
    }

    #[test]
    fn test_request_builder() {
        let data = serde_json::json!({"test": "data"});
        let req = HttpRequest::builder("https://example.com", Method::POST)
            .header("Authorization", "Bearer token")
            .body(RequestBody::json(&data).unwrap())
            .build();

        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.method, Method::POST);
        assert_eq!(req.headers.get("authorization").unwrap(), "Bearer token");
        assert_eq!(req.headers.get("content-type").unwrap(), "application/json");
        assert!(req.body.is_some());
    }
}


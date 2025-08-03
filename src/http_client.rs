use async_trait::async_trait;
use http::{HeaderMap, Method};
use std::fmt;

/// Parse a string into an HTTP method
///
/// Supports the standard HTTP methods commonly used in REST APIs and web development.
/// Each method has specific semantics according to HTTP specifications.
///
/// # Examples
///
/// ```
/// use wave::http_client::parse_method;
/// use http::Method;
///
/// assert_eq!(parse_method("GET").unwrap(), Method::GET);
/// assert_eq!(parse_method("post").unwrap(), Method::POST);
/// ```
pub fn parse_method(s: &str) -> Result<Method, HttpError> {
    match s.to_uppercase().as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        "PUT" => Ok(Method::PUT),
        "DELETE" => Ok(Method::DELETE),
        "PATCH" => Ok(Method::PATCH),
        "HEAD" => Ok(Method::HEAD),
        "OPTIONS" => Ok(Method::OPTIONS),
        _ => Err(HttpError::UnsupportedMethod(s.to_string())),
    }
}

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

/// Represents different types of request bodies with automatic serialization
///
/// Provides type-safe handling of various request body formats with automatic
/// Content-Type header management and proper encoding for each format.
///
/// # Examples
///
/// ```
/// use wave::http_client::RequestBody;
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
    Form(Vec<(String, String)>),
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
    /// use wave::http_client::RequestBody;
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
    /// use wave::http_client::RequestBody;
    ///
    /// let form_data = vec![
    ///     ("username".to_string(), "alice".to_string()),
    ///     ("password".to_string(), "secret123".to_string()),
    /// ];
    /// let body = RequestBody::form(form_data);
    /// ```
    pub fn form(data: Vec<(String, String)>) -> Self {
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
/// use wave::http_client::{RequestBuilder, RequestBody};
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
            key_str.parse::<http::HeaderName>(),
            value_str.parse::<http::HeaderValue>(),
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
    pub fn headers_from_vec(mut self, headers: Vec<(String, String)>) -> Self {
        for (key, value) in headers {
            if let (Ok(header_name), Ok(header_value)) = (
                key.parse::<http::HeaderName>(),
                value.parse::<http::HeaderValue>(),
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
    /// use wave::http_client::{RequestBuilder, RequestBody};
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

/// Represents an HTTP response with status, headers, and body
///
/// Contains all the information returned by an HTTP server, including utilities
/// for parsing common response formats and checking status codes.
///
/// # Examples
///
/// ```
/// use wave::http_client::HttpResponse;
/// use http::HeaderMap;
///
/// let mut headers = HeaderMap::new();
/// headers.insert("content-type", "application/json".parse().unwrap());
///
/// let response = HttpResponse {
///     status: 200,
///     headers,
///     body: r#"{"message": "success"}"#.to_string(),
/// };
///
/// assert!(response.is_success());
/// assert!(response.is_json());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct HttpResponse {
    /// HTTP status code (200, 404, 500, etc.)
    pub status: u16,
    /// Response headers
    pub headers: HeaderMap,
    /// Response body as string
    pub body: String,
}

impl HttpResponse {
    /// Returns true if the response status indicates success (2xx)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Returns true if the response status indicates a client error (4xx)
    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    /// Returns true if the response status indicates a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        self.status >= 500 && self.status < 600
    }

    /// Returns true if the response status indicates any error (4xx or 5xx)
    pub fn is_error(&self) -> bool {
        self.status >= 400
    }

    /// Returns the Content-Type header value, if present
    pub fn content_type(&self) -> Option<&str> {
        self.headers
            .get("content-type")
            .and_then(|value| value.to_str().ok())
    }

    /// Parse the response body as JSON
    ///
    /// Attempts to deserialize the response body into the specified type.
    /// Returns an error if the body is not valid JSON or doesn't match the expected structure.
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::http_client::HttpResponse;
    /// use serde::Deserialize;
    /// use http::HeaderMap;
    ///
    /// #[derive(Deserialize)]
    /// struct User {
    ///     name: String,
    ///     email: String,
    /// }
    ///
    /// let response = HttpResponse {
    ///     status: 200,
    ///     headers: HeaderMap::new(),
    ///     body: r#"{"name": "Alice", "email": "alice@example.com"}"#.to_string(),
    /// };
    ///
    /// let user: User = response.json()?;
    /// assert_eq!(user.name, "Alice");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, HttpError> {
        serde_json::from_str(&self.body)
            .map_err(|e| HttpError::Parse(format!("Failed to parse JSON response: {e}")))
    }

    /// Get the response body as a string reference
    pub fn text(&self) -> &str {
        &self.body
    }

    /// Returns true if the response body appears to be JSON based on Content-Type header
    pub fn is_json(&self) -> bool {
        self.content_type()
            .map(|ct| ct.contains("application/json") || ct.contains("text/json"))
            .unwrap_or(false)
    }
}

/// Represents an HTTP request with method, URL, headers, and optional body
///
/// The core request type used throughout the wave HTTP client. Can be constructed
/// directly or through the builder pattern for more complex scenarios.
///
/// # Examples
///
/// ```
/// use wave::http_client::{HttpRequest, RequestBody};
/// use http::{HeaderMap, Method};
///
/// // Simple GET request
/// let request = HttpRequest::new(
///     "https://api.example.com/users",
///     Method::GET,
///     None,
///     HeaderMap::new()
/// );
///
/// // Using the builder pattern for complex requests
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
    /// use wave::http_client::HttpRequest;
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
    /// use wave::http_client::{HttpRequest, RequestBody};
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

/// Trait for HTTP backends that handle the actual network communication
///
/// This trait allows the HTTP client to be backend-agnostic, enabling
/// different implementations for production (reqwest), testing (mock), or
/// other specialized use cases.
///
/// # Examples
///
/// ```
/// use wave::http_client::{HttpBackend, HttpRequest, HttpResponse, HttpError};
/// use async_trait::async_trait;
///
/// struct LoggingBackend<B: HttpBackend> {
///     inner: B,
/// }
///
/// #[async_trait]
/// impl<B: HttpBackend + Send + Sync> HttpBackend for LoggingBackend<B> {
///     async fn send(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError> {
///         println!("Sending request to: {}", req.url);
///         let response = self.inner.send(req).await?;
///         println!("Received response with status: {}", response.status);
///         Ok(response)
///     }
/// }
/// ```
#[async_trait]
pub trait HttpBackend {
    /// Send an HTTP request and return the response
    ///
    /// This is the core method that implementations must provide to handle
    /// the actual HTTP communication.
    async fn send(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError>;
}

/// Default backend using reqwest for real HTTP requests
///
/// This is the production backend that performs actual network communication
/// using the reqwest library. It handles all standard HTTP methods and
/// automatically manages connection pooling, timeouts, and other network concerns.
pub struct ReqwestBackend;

#[async_trait]
impl HttpBackend for ReqwestBackend {
    async fn send(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError> {
        let client = reqwest::Client::new();
        let mut request_builder = match &req.method {
            &Method::GET => client.get(&req.url),
            &Method::POST => client.post(&req.url),
            &Method::PUT => client.put(&req.url),
            &Method::DELETE => client.delete(&req.url),
            &Method::PATCH => client.patch(&req.url),
            &Method::HEAD => client.head(&req.url),
            &Method::OPTIONS => client.request(reqwest::Method::OPTIONS, &req.url),
            method => client.request(
                reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap(),
                &req.url,
            ),
        };
        if let Some(ref body) = req.body {
            request_builder = request_builder.body(body.clone());
        }
        // Set headers
        for (key, value) in &req.headers {
            request_builder = request_builder.header(key.as_str(), value.to_str().unwrap_or(""));
        }
        let resp = request_builder
            .send()
            .await
            .map_err(|e| HttpError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        let mut headers = HeaderMap::new();
        for (k, v) in resp.headers() {
            headers.insert(k.clone(), v.clone());
        }
        let body = resp
            .text()
            .await
            .map_err(|e| HttpError::Parse(e.to_string()))?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// HTTP client generic over backend
///
/// A flexible HTTP client that can work with any backend implementing the
/// `HttpBackend` trait. Use `ReqwestBackend` for real network requests,
/// or implement a custom backend for testing or specialized behavior.
///
/// # Examples
///
/// ```
/// use wave::http_client::{Client, ReqwestBackend, HttpRequest};
/// use http::{HeaderMap, Method};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new(ReqwestBackend);
/// let request = HttpRequest::new(
///     "https://httpbin.org/get",
///     Method::GET,
///     None,
///     HeaderMap::new()
/// );
///
/// let response = client.send(&request).await?;
/// println!("Status: {}", response.status);
/// # Ok(())
/// # }
/// ```
///
/// Use ReqwestBackend for real requests, or a mock for tests.
#[derive(Clone)]
pub struct Client<B: HttpBackend + Send + Sync> {
    pub backend: B,
}

impl<B: HttpBackend + Send + Sync> Client<B> {
    /// Constructs a new Client with the given backend
    ///
    /// Creates a client instance that will use the specified backend for
    /// all HTTP operations. The backend determines how requests are actually sent.
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    /// Sends an HTTP request and returns the response
    ///
    /// This is the main method for executing HTTP requests. It delegates to the
    /// configured backend to perform the actual network communication.
    ///
    /// # Examples
    ///
    /// ```
    /// use wave::http_client::{Client, ReqwestBackend, HttpRequest};
    /// use http::{HeaderMap, Method};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::new(ReqwestBackend);
    /// let request = HttpRequest::builder("https://httpbin.org/get", Method::GET)
    ///     .header("User-Agent", "wave/1.0")
    ///     .build();
    ///
    /// match client.send(&request).await {
    ///     Ok(response) if response.is_success() => {
    ///         println!("Success: {}", response.body);
    ///     }
    ///     Ok(response) => {
    ///         println!("HTTP error: {}", response.status);
    ///     }
    ///     Err(e) => {
    ///         println!("Network error: {}", e);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError> {
        self.backend.send(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use tokio_test::block_on;

    struct MockBackend {
        pub last_request: Mutex<Option<HttpRequest>>,
        pub response: HttpResponse,
        pub error: Option<HttpError>,
    }

    #[async_trait]
    impl HttpBackend for MockBackend {
        async fn send(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError> {
            let mut last = self.last_request.lock().unwrap();
            *last = Some(req.clone());
            if let Some(ref err) = self.error {
                Err(err.clone())
            } else {
                Ok(self.response.clone())
            }
        }
    }

    // Implementing HttpBackend for Arc<MockBackend> allows us to pass an Arc-wrapped backend to the Client,
    // which takes ownership of its backend. This enables tests to retain a reference to the same backend instance
    // even after passing it to the client, so we can inspect and assert on properties like `last_request` after
    // the client has made a request. Without this, tests could only check the response, not verify that the client
    // constructed and sent the correct request (URL, method, headers, body).
    #[async_trait]
    impl HttpBackend for Arc<MockBackend> {
        async fn send(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError> {
            self.as_ref().send(req).await
        }
    }

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
    fn test_parse_method() {
        // Test: Valid method parsing - unwrap is safe for known valid strings
        assert_eq!(parse_method("GET").expect("Test: Valid GET"), Method::GET);
        assert_eq!(parse_method("get").expect("Test: Valid get"), Method::GET);
        assert_eq!(
            parse_method("POST").expect("Test: Valid POST"),
            Method::POST
        );
        assert_eq!(parse_method("put").expect("Test: Valid put"), Method::PUT);
        assert_eq!(
            parse_method("DELETE").expect("Test: Valid DELETE"),
            Method::DELETE
        );
        assert_eq!(
            parse_method("patch").expect("Test: Valid patch"),
            Method::PATCH
        );
        assert_eq!(
            parse_method("HEAD").expect("Test: Valid HEAD"),
            Method::HEAD
        );
        assert_eq!(
            parse_method("options").expect("Test: Valid options"),
            Method::OPTIONS
        );

        assert!(matches!(
            parse_method("INVALID"),
            Err(HttpError::UnsupportedMethod(_))
        ));
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
    fn test_client_get_calls_backend_and_returns_response() {
        let mut expected_headers = HeaderMap::new();
        expected_headers.insert("x-resp", "ok".parse().unwrap());

        let backend = Arc::new(MockBackend {
            last_request: Mutex::new(None),
            response: HttpResponse {
                status: 200,
                headers: expected_headers,
                body: "hello".to_string(),
            },
            error: None,
        });
        let client = Client::new(backend.clone());
        let req = HttpRequest::builder("http://test", Method::GET)
            .header("X-Req", "1")
            .build();
        let resp = block_on(client.send(&req)).unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "hello");
        let req = backend.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(req.url, "http://test");
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.headers.get("x-req").unwrap(), "1");
    }

    #[test]
    fn test_client_post_calls_backend_and_returns_response() {
        let backend = Arc::new(MockBackend {
            last_request: Mutex::new(None),
            response: HttpResponse {
                status: 201,
                headers: HeaderMap::new(),
                body: "created".to_string(),
            },
            error: None,
        });
        let client = Client::new(backend.clone());
        let req = HttpRequest::new(
            "http://test",
            Method::POST,
            Some("payload".to_string()),
            HeaderMap::new(),
        );
        let resp = block_on(client.send(&req)).unwrap();
        assert_eq!(resp.status, 201);
        assert_eq!(resp.body, "created");
        let req = backend.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(req.method, Method::POST);
        assert_eq!(req.body, Some("payload".to_string()));
    }

    #[test]
    fn test_client_handles_backend_error() {
        let backend = Arc::new(MockBackend {
            last_request: Mutex::new(None),
            response: HttpResponse {
                status: 500,
                headers: HeaderMap::new(),
                body: "fail".to_string(),
            },
            error: Some(HttpError::Network("mock error".to_string())),
        });
        let client = Client::new(backend.clone());
        let req = HttpRequest::new("http://fail", Method::GET, None, HeaderMap::new());
        let err = block_on(client.send(&req)).unwrap_err();
        assert!(matches!(err, HttpError::Network(_)));
        assert_eq!(format!("{err}"), "Network error: mock error");
        let req = backend.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(req.url, "http://fail");
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

    // Tests for HttpResponse convenience methods
    #[test]
    fn test_response_status_methods() {
        // Test is_success (2xx range)
        let resp_200 = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: "OK".to_string(),
        };
        let resp_201 = HttpResponse {
            status: 201,
            headers: HeaderMap::new(),
            body: "Created".to_string(),
        };
        let resp_299 = HttpResponse {
            status: 299,
            headers: HeaderMap::new(),
            body: "Custom success".to_string(),
        };
        let resp_300 = HttpResponse {
            status: 300,
            headers: HeaderMap::new(),
            body: "Redirect".to_string(),
        };

        assert!(resp_200.is_success());
        assert!(resp_201.is_success());
        assert!(resp_299.is_success());
        assert!(!resp_300.is_success());

        // Test is_client_error (4xx range)
        let resp_399 = HttpResponse {
            status: 399,
            headers: HeaderMap::new(),
            body: "Custom redirect".to_string(),
        };
        let resp_400 = HttpResponse {
            status: 400,
            headers: HeaderMap::new(),
            body: "Bad request".to_string(),
        };
        let resp_404 = HttpResponse {
            status: 404,
            headers: HeaderMap::new(),
            body: "Not found".to_string(),
        };
        let resp_499 = HttpResponse {
            status: 499,
            headers: HeaderMap::new(),
            body: "Custom client error".to_string(),
        };
        let resp_500 = HttpResponse {
            status: 500,
            headers: HeaderMap::new(),
            body: "Server error".to_string(),
        };

        assert!(!resp_200.is_client_error());
        assert!(!resp_399.is_client_error());
        assert!(resp_400.is_client_error());
        assert!(resp_404.is_client_error());
        assert!(resp_499.is_client_error());
        assert!(!resp_500.is_client_error());

        // Test is_server_error (5xx range)
        let resp_502 = HttpResponse {
            status: 502,
            headers: HeaderMap::new(),
            body: "Bad gateway".to_string(),
        };
        let resp_599 = HttpResponse {
            status: 599,
            headers: HeaderMap::new(),
            body: "Custom server error".to_string(),
        };
        let resp_600 = HttpResponse {
            status: 600,
            headers: HeaderMap::new(),
            body: "Custom status".to_string(),
        };

        assert!(!resp_404.is_server_error());
        assert!(!resp_499.is_server_error());
        assert!(resp_500.is_server_error());
        assert!(resp_502.is_server_error());
        assert!(resp_599.is_server_error());
        assert!(!resp_600.is_server_error());

        // Test is_error (4xx or 5xx range)
        assert!(!resp_200.is_error());
        assert!(!resp_399.is_error());
        assert!(resp_400.is_error());
        assert!(resp_500.is_error());
    }

    #[test]
    fn test_response_content_type() {
        let mut headers_json = HeaderMap::new();
        headers_json.insert(
            "content-type",
            http::HeaderValue::from_static("application/json; charset=utf-8"),
        );

        let mut headers_html = HeaderMap::new();
        headers_html.insert("content-type", http::HeaderValue::from_static("text/html"));

        let resp_json = HttpResponse {
            status: 200,
            headers: headers_json,
            body: "{}".to_string(),
        };
        let resp_html = HttpResponse {
            status: 200,
            headers: headers_html,
            body: "<html></html>".to_string(),
        };
        let resp_no_header = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: "plain text".to_string(),
        };

        assert_eq!(
            resp_json.content_type(),
            Some("application/json; charset=utf-8")
        );
        assert_eq!(resp_html.content_type(), Some("text/html"));
        assert_eq!(resp_no_header.content_type(), None);
    }

    #[test]
    fn test_response_is_json() {
        let mut headers_json = HeaderMap::new();
        headers_json.insert(
            "content-type",
            http::HeaderValue::from_static("application/json"),
        );

        let mut headers_json_charset = HeaderMap::new();
        headers_json_charset.insert(
            "content-type",
            http::HeaderValue::from_static("application/json; charset=utf-8"),
        );

        let mut headers_text_json = HeaderMap::new();
        headers_text_json.insert("content-type", http::HeaderValue::from_static("text/json"));

        let mut headers_html = HeaderMap::new();
        headers_html.insert("content-type", http::HeaderValue::from_static("text/html"));

        let resp_json = HttpResponse {
            status: 200,
            headers: headers_json,
            body: "{}".to_string(),
        };
        let resp_json_charset = HttpResponse {
            status: 200,
            headers: headers_json_charset,
            body: "{}".to_string(),
        };
        let resp_text_json = HttpResponse {
            status: 200,
            headers: headers_text_json,
            body: "{}".to_string(),
        };
        let resp_html = HttpResponse {
            status: 200,
            headers: headers_html,
            body: "<html></html>".to_string(),
        };
        let resp_no_header = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: "{}".to_string(),
        };

        assert!(resp_json.is_json());
        assert!(resp_json_charset.is_json());
        assert!(resp_text_json.is_json());
        assert!(!resp_html.is_json());
        assert!(!resp_no_header.is_json());
    }

    #[test]
    fn test_response_json_parsing() {
        use serde::Deserialize;

        #[derive(Deserialize, PartialEq, Debug)]
        struct TestData {
            name: String,
            age: u32,
        }

        let resp_valid_json = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: r#"{"name": "Alice", "age": 30}"#.to_string(),
        };
        let resp_invalid_json = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: "not json".to_string(),
        };
        let resp_wrong_schema = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: r#"{"wrong": "schema"}"#.to_string(),
        };

        // Test successful parsing
        let parsed: TestData = resp_valid_json.json().unwrap();
        assert_eq!(parsed.name, "Alice");
        assert_eq!(parsed.age, 30);

        // Test serde_json::Value parsing
        let json_value: serde_json::Value = resp_valid_json.json().unwrap();
        assert_eq!(json_value["name"], "Alice");
        assert_eq!(json_value["age"], 30);

        // Test parsing invalid JSON
        let result: Result<TestData, _> = resp_invalid_json.json();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HttpError::Parse(_)));

        // Test parsing JSON with wrong schema
        let result: Result<TestData, _> = resp_wrong_schema.json();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), HttpError::Parse(_)));
    }

    #[test]
    fn test_response_text() {
        let resp = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: "Hello, World!".to_string(),
        };

        assert_eq!(resp.text(), "Hello, World!");
        assert_eq!(resp.text(), &resp.body); // Ensure it's the same reference
    }
}

use async_trait::async_trait;
use http::HeaderMap;
use std::fmt;

/// HTTP methods supported by the client
#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Options => write!(f, "OPTIONS"),
        }
    }
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Result<Self, HttpError> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "POST" => Ok(HttpMethod::Post),
            "PUT" => Ok(HttpMethod::Put),
            "DELETE" => Ok(HttpMethod::Delete),
            "PATCH" => Ok(HttpMethod::Patch),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            _ => Err(HttpError::UnsupportedMethod(s.to_string())),
        }
    }
}

/// Custom error types for HTTP operations
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
            HttpError::Network(msg) => write!(f, "Network error: {}", msg),
            HttpError::Parse(msg) => write!(f, "Parse error: {}", msg),
            HttpError::UnsupportedMethod(method) => write!(f, "Unsupported HTTP method: {}", method),
            HttpError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for HttpError {}

/// Represents different types of request bodies
#[derive(Debug, Clone)]
pub enum RequestBody {
    Json(serde_json::Value),
    Form(Vec<(String, String)>),
    Text(String),
    Bytes(Vec<u8>),
}

impl RequestBody {
    /// Create a JSON body from any serializable type
    pub fn json<T: serde::Serialize>(data: &T) -> Result<Self, HttpError> {
        let value = serde_json::to_value(data)
            .map_err(|e| HttpError::Parse(format!("Failed to serialize JSON: {}", e)))?;
        Ok(RequestBody::Json(value))
    }
    
    /// Create a form-encoded body
    pub fn form(data: Vec<(String, String)>) -> Self {
        RequestBody::Form(data)
    }
    
    /// Create a plain text body
    pub fn text(data: String) -> Self {
        RequestBody::Text(data)
    }
    
    /// Create a binary body
    pub fn bytes(data: Vec<u8>) -> Self {
        RequestBody::Bytes(data)
    }
    
    /// Serialize the body to a string and set appropriate Content-Type header
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
#[derive(Debug)]
pub struct RequestBuilder {
    url: String,
    method: HttpMethod,
    headers: HeaderMap,
    body: Option<RequestBody>,
}

impl RequestBuilder {
    /// Create a new request builder
    pub fn new(url: impl Into<String>, method: HttpMethod) -> Self {
        Self {
            url: url.into(),
            method,
            headers: HeaderMap::new(),
            body: None,
        }
    }
    
    /// Add a header to the request
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key_str = key.into();
        let value_str = value.into();
        if let (Ok(header_name), Ok(header_value)) = (key_str.parse::<http::HeaderName>(), value_str.parse::<http::HeaderValue>()) {
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
            if let (Ok(header_name), Ok(header_value)) = (key.parse::<http::HeaderName>(), value.parse::<http::HeaderValue>()) {
                self.headers.insert(header_name, header_value);
            }
        }
        self
    }
    
    /// Set a JSON body from any serializable type
    pub fn json_body<T: serde::Serialize>(mut self, data: &T) -> Result<Self, HttpError> {
        self.body = Some(RequestBody::json(data)?);
        Ok(self)
    }
    
    /// Set a form-encoded body
    pub fn form_body(mut self, data: Vec<(String, String)>) -> Self {
        self.body = Some(RequestBody::form(data));
        self
    }
    
    /// Set a plain text body
    pub fn text_body(mut self, data: String) -> Self {
        self.body = Some(RequestBody::text(data));
        self
    }
    
    /// Set a binary body
    pub fn bytes_body(mut self, data: Vec<u8>) -> Self {
        self.body = Some(RequestBody::bytes(data));
        self
    }
    
    /// Set a raw body (for backward compatibility or custom handling)
    pub fn raw_body(mut self, data: String) -> Self {
        self.body = Some(RequestBody::text(data));
        self
    }
    
    /// Build the final HttpRequest
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

/// Represents an HTTP response.
#[derive(Clone, Debug, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: String,
}

/// Represents an HTTP request.
#[derive(Debug, PartialEq, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub method: HttpMethod,
    pub body: Option<String>,
    pub headers: HeaderMap,
}

impl HttpRequest {
    /// Constructs a new HttpRequest with HeaderMap
    pub fn new(
        url: &str,
        method: HttpMethod,
        body: Option<String>,
        headers: HeaderMap,
    ) -> Self {
        Self {
            url: url.to_string(),
            method,
            body,
            headers,
        }
    }
    
    /// Constructs a new HttpRequest from Vec<(String, String)> (convenience method for backward compatibility)
    pub fn new_with_headers(
        url: &str,
        method: HttpMethod,
        body: Option<String>,
        headers: Vec<(String, String)>,
    ) -> Self {
        let mut header_map = HeaderMap::new();
        for (key, value) in headers {
            if let (Ok(header_name), Ok(header_value)) = (key.parse::<http::HeaderName>(), value.parse::<http::HeaderValue>()) {
                header_map.insert(header_name, header_value);
            }
        }
        
        Self {
            url: url.to_string(),
            method,
            body,
            headers: header_map,
        }
    }
    
    /// Constructs a new HttpRequest with a RequestBody that handles serialization
    pub fn with_body(
        url: &str,
        method: HttpMethod,
        body: Option<RequestBody>,
        headers: HeaderMap,
    ) -> Self {
        let mut headers = headers;
        let body_string = body.map(|b| b.serialize(&mut headers));
        
        Self {
            url: url.to_string(),
            method,
            body: body_string,
            headers,
        }
    }
    
    /// Constructs a new HttpRequest with RequestBody from Vec<(String, String)> (convenience method)
    pub fn with_body_from_headers(
        url: &str,
        method: HttpMethod,
        body: Option<RequestBody>,
        headers: Vec<(String, String)>,
    ) -> Self {
        let mut header_map = HeaderMap::new();
        for (key, value) in headers {
            if let (Ok(header_name), Ok(header_value)) = (key.parse::<http::HeaderName>(), value.parse::<http::HeaderValue>()) {
                header_map.insert(header_name, header_value);
            }
        }
        
        Self::with_body(url, method, body, header_map)
    }
    
    /// Create a request builder for complex requests
    pub fn builder(url: impl Into<String>, method: HttpMethod) -> RequestBuilder {
        RequestBuilder::new(url, method)
    }
}

/// Trait for HTTP backends.
#[async_trait]
pub trait HttpBackend {
    async fn send(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError>;
}

/// Default backend using reqwest for real HTTP requests.
pub struct ReqwestBackend;

#[async_trait]
impl HttpBackend for ReqwestBackend {
    async fn send(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError> {
        let client = reqwest::Client::new();
        let mut request_builder = match req.method {
            HttpMethod::Get => client.get(&req.url),
            HttpMethod::Post => client.post(&req.url),
            HttpMethod::Put => client.put(&req.url),
            HttpMethod::Delete => client.delete(&req.url),
            HttpMethod::Patch => client.patch(&req.url),
            HttpMethod::Head => client.head(&req.url),
            HttpMethod::Options => client.request(reqwest::Method::OPTIONS, &req.url),
        };
        if let Some(ref body) = req.body {
            request_builder = request_builder.body(body.clone());
        }
        // Set headers
        for (key, value) in &req.headers {
            request_builder = request_builder.header(key.as_str(), value.to_str().unwrap_or(""));
        }
        let resp = request_builder.send().await
            .map_err(|e| HttpError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        let mut headers = HeaderMap::new();
        for (k, v) in resp.headers() {
            headers.insert(k.clone(), v.clone());
        }
        let body = resp.text().await
            .map_err(|e| HttpError::Parse(e.to_string()))?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

/// HTTP client generic over backend. Use ReqwestBackend for real requests, or a mock for tests.
#[derive(Clone)]
pub struct Client<B: HttpBackend + Send + Sync> {
    pub backend: B,
}

impl<B: HttpBackend + Send + Sync> Client<B> {
    /// Constructs a new Client with the given backend.
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
    
    /// Sends an HTTP request and returns the response
    pub async fn send(&self, req: &HttpRequest) -> Result<HttpResponse, HttpError> {
        self.backend.send(req).await
    }
    
    // Legacy methods for backward compatibility - deprecated but kept for now
    // These will be removed in a future version
    
    /// Encodes key-value pairs as application/x-www-form-urlencoded and sets Content-Type header.
    /// DEPRECATED: Use RequestBody::form() instead
    pub fn prepare_form_body(
        data: &[(String, String)],
        headers: &mut HeaderMap,
    ) -> String {
        let body = RequestBody::form(data.to_vec());
        body.serialize(headers)
    }

    /// Encodes key-value pairs as a JSON object and sets Content-Type header.
    /// DEPRECATED: Use RequestBody::json() instead
    pub fn prepare_json_body(
        data: Vec<(String, String)>,
        headers: &mut HeaderMap,
    ) -> String {
        let map: std::collections::HashMap<String, String> = data.into_iter().collect();
        match RequestBody::json(&map) {
            Ok(body) => body.serialize(headers),
            Err(_) => {
                RequestBody::ensure_content_type(headers, "application/json");
                "{}".to_string()
            }
        }
    }

    /// Sends a GET request.
    /// DEPRECATED: Use Client::send() with HttpRequest::new() or HttpRequest::builder()
    pub async fn get(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new_with_headers(url, HttpMethod::Get, None, headers);
        self.send(&req).await
    }
    
    /// Sends a POST request.
    /// DEPRECATED: Use Client::send() with HttpRequest::new() or HttpRequest::builder()
    pub async fn post(
        &self,
        url: &str,
        body: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new_with_headers(url, HttpMethod::Post, Some(body.to_string()), headers);
        self.send(&req).await
    }
    
    /// Sends a PUT request.
    /// DEPRECATED: Use Client::send() with HttpRequest::new() or HttpRequest::builder()
    pub async fn put(
        &self,
        url: &str,
        body: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new_with_headers(url, HttpMethod::Put, Some(body.to_string()), headers);
        self.send(&req).await
    }
    
    /// Sends a DELETE request.
    /// DEPRECATED: Use Client::send() with HttpRequest::new() or HttpRequest::builder()
    pub async fn delete(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new_with_headers(url, HttpMethod::Delete, None, headers);
        self.send(&req).await
    }
    
    /// Sends a PATCH request.
    /// DEPRECATED: Use Client::send() with HttpRequest::new() or HttpRequest::builder()
    pub async fn patch(
        &self,
        url: &str,
        body: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new_with_headers(url, HttpMethod::Patch, Some(body.to_string()), headers);
        self.send(&req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tokio_test::block_on;

    struct MockBackend {
        pub last_request: std::sync::Mutex<Option<HttpRequest>>,
        pub response: HttpResponse,
        pub error: Option<HttpError>,
    }

    #[async_trait]
    impl HttpBackend for MockBackend {
        async fn send(
            &self,
            req: &HttpRequest,
        ) -> Result<HttpResponse, HttpError> {
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
    impl HttpBackend for std::sync::Arc<MockBackend> {
        async fn send(
            &self,
            req: &HttpRequest,
        ) -> Result<HttpResponse, HttpError> {
            self.as_ref().send(req).await
        }
    }

    #[test]
    fn test_http_request_construction() {
        let mut headers = HeaderMap::new();
        headers.insert("x-test", "1".parse().unwrap());
        
        let req = HttpRequest::new(
            "http://example.com",
            HttpMethod::Post,
            Some("body".to_string()),
            headers.clone(),
        );
        assert_eq!(req.url, "http://example.com");
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.body, Some("body".to_string()));
        assert_eq!(req.headers, headers);
    }

    #[test]
    fn test_http_method_from_str() {
        assert_eq!(HttpMethod::from_str("GET").unwrap(), HttpMethod::Get);
        assert_eq!(HttpMethod::from_str("get").unwrap(), HttpMethod::Get);
        assert_eq!(HttpMethod::from_str("POST").unwrap(), HttpMethod::Post);
        assert_eq!(HttpMethod::from_str("put").unwrap(), HttpMethod::Put);
        assert_eq!(HttpMethod::from_str("DELETE").unwrap(), HttpMethod::Delete);
        assert_eq!(HttpMethod::from_str("patch").unwrap(), HttpMethod::Patch);
        assert_eq!(HttpMethod::from_str("HEAD").unwrap(), HttpMethod::Head);
        assert_eq!(HttpMethod::from_str("options").unwrap(), HttpMethod::Options);
        
        assert!(matches!(HttpMethod::from_str("INVALID"), Err(HttpError::UnsupportedMethod(_))));
    }

    #[test]
    fn test_http_method_display() {
        assert_eq!(format!("{}", HttpMethod::Get), "GET");
        assert_eq!(format!("{}", HttpMethod::Post), "POST");
        assert_eq!(format!("{}", HttpMethod::Put), "PUT");
        assert_eq!(format!("{}", HttpMethod::Delete), "DELETE");
        assert_eq!(format!("{}", HttpMethod::Patch), "PATCH");
        assert_eq!(format!("{}", HttpMethod::Head), "HEAD");
        assert_eq!(format!("{}", HttpMethod::Options), "OPTIONS");
    }

    #[test]
    fn test_prepare_form_body_sets_header_and_encodes() {
        let mut headers = HeaderMap::new();
        let data = vec![
            ("foo".to_string(), "bar baz".to_string()),
            ("qux".to_string(), "1&2".to_string()),
        ];
        let encoded = Client::<MockBackend>::prepare_form_body(&data, &mut headers);
        assert_eq!(encoded, "foo=bar%20baz&qux=1%262");
        assert!(headers.contains_key("content-type"));
        assert_eq!(headers.get("content-type").unwrap(), "application/x-www-form-urlencoded");
    }

    #[test]
    fn test_prepare_json_body_sets_header_and_encodes() {
        let mut headers = HeaderMap::new();
        let data = vec![
            ("foo".to_string(), "bar".to_string()),
            ("baz".to_string(), "qux".to_string()),
        ];
        let encoded = Client::<MockBackend>::prepare_json_body(data.clone(), &mut headers);
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
        
        let backend = std::sync::Arc::new(MockBackend {
            last_request: std::sync::Mutex::new(None),
            response: HttpResponse {
                status: 200,
                headers: expected_headers,
                body: "hello".to_string(),
            },
            error: None,
        });
        let client = Client::new(backend.clone());
        let resp =
            block_on(client.get("http://test", vec![("X-Req".to_string(), "1".to_string())]))
                .unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "hello");
        let req = backend.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(req.url, "http://test");
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.headers.get("x-req").unwrap(), "1");
    }

    #[test]
    fn test_client_post_calls_backend_and_returns_response() {
        let backend = std::sync::Arc::new(MockBackend {
            last_request: std::sync::Mutex::new(None),
            response: HttpResponse {
                status: 201,
                headers: HeaderMap::new(),
                body: "created".to_string(),
            },
            error: None,
        });
        let client = Client::new(backend.clone());
        let resp = block_on(client.post("http://test", "payload", vec![])).unwrap();
        assert_eq!(resp.status, 201);
        assert_eq!(resp.body, "created");
        let req = backend.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.body, Some("payload".to_string()));
    }

    #[test]
    fn test_request_body_json() {
        let data = serde_json::json!({"name": "Alice", "age": 30});
        let body = RequestBody::json(&data).unwrap();
        let mut headers = HeaderMap::new();
        let serialized = body.serialize(&mut headers);
        
        assert!(headers.contains_key("content-type"));
        assert_eq!(headers.get("content-type").unwrap(), "application/json");
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 30);
    }

    #[test]
    fn test_request_body_form() {
        let data = vec![("name".to_string(), "Alice".to_string()), ("age".to_string(), "30".to_string())];
        let body = RequestBody::form(data);
        let mut headers = HeaderMap::new();
        let serialized = body.serialize(&mut headers);
        
        assert!(headers.contains_key("content-type"));
        assert_eq!(headers.get("content-type").unwrap(), "application/x-www-form-urlencoded");
        assert_eq!(serialized, "name=Alice&age=30");
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
        let req = HttpRequest::builder("https://example.com", HttpMethod::Post)
            .header("Authorization", "Bearer token")
            .json_body(&data)
            .unwrap()
            .build();
        
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.headers.get("authorization").unwrap(), "Bearer token");
        assert_eq!(req.headers.get("content-type").unwrap(), "application/json");
        assert!(req.body.is_some());
    }

    #[test]
    fn test_http_request_with_body() {
        let data = vec![("key".to_string(), "value".to_string())];
        let body = RequestBody::form(data);
        let req = HttpRequest::with_body_from_headers("https://example.com", HttpMethod::Post, Some(body), vec![]);
        
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.headers.get("content-type").unwrap(), "application/x-www-form-urlencoded");
        assert_eq!(req.body, Some("key=value".to_string()));
    }

    #[test]
    fn test_client_send() {
        let backend = std::sync::Arc::new(MockBackend {
            last_request: std::sync::Mutex::new(None),
            response: HttpResponse {
                status: 200,
                headers: HeaderMap::new(),
                body: "success".to_string(),
            },
            error: None,
        });
        let client = Client::new(backend.clone());
        let req = HttpRequest::new_with_headers("http://test", HttpMethod::Get, None, vec![]);
        let resp = block_on(client.send(&req)).unwrap();
        
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, "success");
        let sent_req = backend.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(sent_req.url, "http://test");
        assert_eq!(sent_req.method, HttpMethod::Get);
    }

    #[test]
    fn test_client_handles_backend_error() {
        let backend = std::sync::Arc::new(MockBackend {
            last_request: std::sync::Mutex::new(None),
            response: HttpResponse {
                status: 500,
                headers: HeaderMap::new(),
                body: "fail".to_string(),
            },
            error: Some(HttpError::Network("mock error".to_string())),
        });
        let client = Client::new(backend.clone());
        let err = block_on(client.get("http://fail", vec![])).unwrap_err();
        assert!(matches!(err, HttpError::Network(_)));
        assert_eq!(format!("{err}"), "Network error: mock error");
        let req = backend.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(req.url, "http://fail");
    }
}
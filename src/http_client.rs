use async_trait::async_trait;
use std::fmt;

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

/// Represents an HTTP response.
#[derive(Clone, Debug, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// Represents an HTTP request.
#[derive(Debug, PartialEq, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub method: String,
    pub body: Option<String>,
    pub headers: Vec<(String, String)>,
}

impl HttpRequest {
    /// Constructs a new HttpRequest.
    pub fn new(
        url: &str,
        method: &str,
        body: Option<String>,
        headers: Vec<(String, String)>,
    ) -> Self {
        Self {
            url: url.to_string(),
            method: method.to_string(),
            body,
            headers,
        }
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
        let mut request_builder = match req.method.as_str() {
            "GET" => client.get(&req.url),
            "POST" => client.post(&req.url),
            "PUT" => client.put(&req.url),
            "DELETE" => client.delete(&req.url),
            "PATCH" => client.patch(&req.url),
            _ => return Err(HttpError::UnsupportedMethod(req.method.clone())),
        };
        if let Some(ref body) = req.body {
            request_builder = request_builder.body(body.clone());
        }
        // Set headers
        for (key, value) in &req.headers {
            request_builder = request_builder.header(key, value);
        }
        let resp = request_builder.send().await
            .map_err(|e| HttpError::Network(e.to_string()))?;
        let status = resp.status().as_u16();
        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| {
                let value_str = v.to_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| format!("{:?}", v.as_bytes()));
                (k.to_string(), value_str)
            })
            .collect();
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
    /// Encodes key-value pairs as application/x-www-form-urlencoded and sets Content-Type header.
    pub fn prepare_form_body(
        data: &[(String, String)],
        headers: &mut Vec<(String, String)>,
    ) -> String {
        let encoded = data
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        if !headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            headers.push((
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            ));
        }
        encoded
    }

    /// Encodes key-value pairs as a JSON object and sets Content-Type header.
    pub fn prepare_json_body(
        data: Vec<(String, String)>,
        headers: &mut Vec<(String, String)>,
    ) -> String {
        let map: std::collections::HashMap<String, String> = data.into_iter().collect();
        if !headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        {
            headers.push(("Content-Type".to_string(), "application/json".to_string()));
        }
        serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
    }

    /// Constructs a new Client with the given backend.
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
    /// Sends a GET request.
    pub async fn get(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new(url, "GET", None, headers);
        self.backend.send(&req).await
    }
    /// Sends a POST request.
    pub async fn post(
        &self,
        url: &str,
        body: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new(url, "POST", Some(body.to_string()), headers);
        self.backend.send(&req).await
    }
    /// Sends a PUT request.
    pub async fn put(
        &self,
        url: &str,
        body: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new(url, "PUT", Some(body.to_string()), headers);
        self.backend.send(&req).await
    }
    /// Sends a DELETE request.
    pub async fn delete(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new(url, "DELETE", None, headers);
        self.backend.send(&req).await
    }
    /// Sends a PATCH request.
    pub async fn patch(
        &self,
        url: &str,
        body: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, HttpError> {
        let req = HttpRequest::new(url, "PATCH", Some(body.to_string()), headers);
        self.backend.send(&req).await
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
        let req = HttpRequest::new(
            "http://example.com",
            "POST",
            Some("body".to_string()),
            vec![("X-Test".to_string(), "1".to_string())],
        );
        assert_eq!(req.url, "http://example.com");
        assert_eq!(req.method, "POST");
        assert_eq!(req.body, Some("body".to_string()));
        assert_eq!(req.headers, vec![("X-Test".to_string(), "1".to_string())]);
    }

    #[test]
    fn test_prepare_form_body_sets_header_and_encodes() {
        let mut headers = vec![];
        let data = vec![
            ("foo".to_string(), "bar baz".to_string()),
            ("qux".to_string(), "1&2".to_string()),
        ];
        let encoded = Client::<MockBackend>::prepare_form_body(&data, &mut headers);
        assert_eq!(encoded, "foo=bar%20baz&qux=1%262");
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/x-www-form-urlencoded"));
    }

    #[test]
    fn test_prepare_json_body_sets_header_and_encodes() {
        let mut headers = vec![];
        let data = vec![
            ("foo".to_string(), "bar".to_string()),
            ("baz".to_string(), "qux".to_string()),
        ];
        let encoded = Client::<MockBackend>::prepare_json_body(data.clone(), &mut headers);
        let encoded_json: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        let expected_json: serde_json::Value = serde_json::json!({"foo": "bar", "baz": "qux"});
        assert_eq!(encoded_json, expected_json);
        assert!(headers
            .iter()
            .any(|(k, v)| k == "Content-Type" && v == "application/json"));
    }

    #[test]
    fn test_client_get_calls_backend_and_returns_response() {
        let backend = std::sync::Arc::new(MockBackend {
            last_request: std::sync::Mutex::new(None),
            response: HttpResponse {
                status: 200,
                headers: vec![("X-Resp".to_string(), "ok".to_string())],
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
        assert_eq!(req.method, "GET");
        assert_eq!(req.headers, vec![("X-Req".to_string(), "1".to_string())]);
    }

    #[test]
    fn test_client_post_calls_backend_and_returns_response() {
        let backend = std::sync::Arc::new(MockBackend {
            last_request: std::sync::Mutex::new(None),
            response: HttpResponse {
                status: 201,
                headers: vec![],
                body: "created".to_string(),
            },
            error: None,
        });
        let client = Client::new(backend.clone());
        let resp = block_on(client.post("http://test", "payload", vec![])).unwrap();
        assert_eq!(resp.status, 201);
        assert_eq!(resp.body, "created");
        let req = backend.last_request.lock().unwrap().clone().unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.body, Some("payload".to_string()));
    }

    #[test]
    fn test_client_handles_backend_error() {
        let backend = std::sync::Arc::new(MockBackend {
            last_request: std::sync::Mutex::new(None),
            response: HttpResponse {
                status: 500,
                headers: vec![],
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
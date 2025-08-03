use crate::http::{
    backend::HttpBackend, error::HttpError, request::HttpRequest, response::HttpResponse,
};

/// HTTP client generic over backend
///
/// A flexible HTTP client that can work with any backend implementing the
/// `HttpBackend` trait. Use `ReqwestBackend` for real network requests,
/// or implement a custom backend for testing or specialized behavior.
///
/// # Examples
///
/// ```
/// use wave::http::{Client, ReqwestBackend, HttpRequest};
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
    /// use wave::http::{Client, ReqwestBackend, HttpRequest};
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
    use ::http::{HeaderMap, Method};
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
    fn test_client_get_calls_backend_and_returns_response() {
        let mut expected_headers = HeaderMap::new();
        expected_headers.insert("x-resp", "ok".parse().unwrap());
        let expected_response = HttpResponse {
            status: 200,
            headers: expected_headers.clone(),
            body: "test body".to_string(),
        };

        let backend = Arc::new(MockBackend {
            last_request: Mutex::new(None),
            response: expected_response.clone(),
            error: None,
        });

        let client = Client::new(backend.clone());
        let req = HttpRequest::new("http://example.com", Method::GET, None, HeaderMap::new());

        let result = block_on(client.send(&req));
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.headers, expected_headers);
        assert_eq!(response.body, "test body");

        // Verify the backend received the correct request
        let last_req = backend.last_request.lock().unwrap();
        assert!(last_req.is_some());
        let sent_req = last_req.as_ref().unwrap();
        assert_eq!(sent_req.url, "http://example.com");
        assert_eq!(sent_req.method, Method::GET);
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
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        let req = HttpRequest::new(
            "http://example.com/api",
            Method::POST,
            Some(r#"{"data":"value"}"#.to_string()),
            headers,
        );

        let result = block_on(client.send(&req));
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, 201);

        // Verify the backend received the POST request with body
        let last_req = backend.last_request.lock().unwrap();
        let sent_req = last_req.as_ref().unwrap();
        assert_eq!(sent_req.method, Method::POST);
        assert_eq!(sent_req.body, Some(r#"{"data":"value"}"#.to_string()));
        assert_eq!(
            sent_req.headers.get("content-type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_client_handles_backend_error() {
        let backend = Arc::new(MockBackend {
            last_request: Mutex::new(None),
            response: HttpResponse {
                status: 500,
                headers: HeaderMap::new(),
                body: "".to_string(),
            },
            error: Some(HttpError::Network("Connection failed".to_string())),
        });

        let client = Client::new(backend);
        let req = HttpRequest::new("http://example.com", Method::GET, None, HeaderMap::new());

        let result = block_on(client.send(&req));
        assert!(result.is_err());
        match result.unwrap_err() {
            HttpError::Network(msg) => assert_eq!(msg, "Connection failed"),
            _ => panic!("Expected HttpError::Network"),
        }
    }
}

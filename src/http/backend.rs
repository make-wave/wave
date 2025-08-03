use crate::http::{error::HttpError, request::HttpRequest, response::HttpResponse};
use ::http::{HeaderMap, Method};
use async_trait::async_trait;

/// Trait for HTTP backends that handle the actual network communication
///
/// This trait allows the HTTP client to be backend-agnostic, enabling
/// different implementations for production (reqwest), testing (mock), or
/// other specialized use cases.
///
/// # Examples
///
/// ```
/// use wave::http::{HttpBackend, HttpRequest, HttpResponse, HttpError};
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

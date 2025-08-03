use crate::http::error::HttpError;
use ::http::HeaderMap;

/// Represents an HTTP response with status, headers, and body
///
/// Contains all the information returned by an HTTP server, including utilities
/// for parsing common response formats and checking status codes.
///
/// # Examples
///
/// ```
/// use wave::http::HttpResponse;
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
    /// use wave::http::HttpResponse;
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(resp_200.is_success());
        assert!(resp_201.is_success());

        // Test is_client_error (4xx range)
        let resp_404 = HttpResponse {
            status: 404,
            headers: HeaderMap::new(),
            body: "Not Found".to_string(),
        };
        assert!(resp_404.is_client_error());
        assert!(resp_404.is_error());
        assert!(!resp_404.is_success());

        // Test is_server_error (5xx range)
        let resp_500 = HttpResponse {
            status: 500,
            headers: HeaderMap::new(),
            body: "Internal Server Error".to_string(),
        };
        assert!(resp_500.is_server_error());
        assert!(resp_500.is_error());
        assert!(!resp_500.is_success());
    }

    #[test]
    fn test_response_content_type() {
        let mut headers_json = HeaderMap::new();
        headers_json.insert(
            "content-type",
            ::http::HeaderValue::from_static("application/json; charset=utf-8"),
        );

        let mut headers_html = HeaderMap::new();
        headers_html.insert(
            "content-type",
            ::http::HeaderValue::from_static("text/html"),
        );

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

        let resp_no_content_type = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: "data".to_string(),
        };

        assert_eq!(
            resp_json.content_type(),
            Some("application/json; charset=utf-8")
        );
        assert_eq!(resp_html.content_type(), Some("text/html"));
        assert_eq!(resp_no_content_type.content_type(), None);
    }

    #[test]
    fn test_response_is_json() {
        let mut headers_json = HeaderMap::new();
        headers_json.insert(
            "content-type",
            ::http::HeaderValue::from_static("application/json"),
        );

        let mut headers_json_charset = HeaderMap::new();
        headers_json_charset.insert(
            "content-type",
            ::http::HeaderValue::from_static("application/json; charset=utf-8"),
        );

        let mut headers_text_json = HeaderMap::new();
        headers_text_json.insert(
            "content-type",
            ::http::HeaderValue::from_static("text/json"),
        );

        let mut headers_html = HeaderMap::new();
        headers_html.insert(
            "content-type",
            ::http::HeaderValue::from_static("text/html"),
        );

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
        let resp_no_headers = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: "{}".to_string(),
        };

        assert!(resp_json.is_json());
        assert!(resp_json_charset.is_json());
        assert!(resp_text_json.is_json());
        assert!(!resp_html.is_json());
        assert!(!resp_no_headers.is_json());
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
            body: "invalid json".to_string(),
        };

        let parsed: Result<TestData, _> = resp_valid_json.json();
        assert!(parsed.is_ok());
        let data = parsed.unwrap();
        assert_eq!(
            data,
            TestData {
                name: "Alice".to_string(),
                age: 30
            }
        );

        let parsed_invalid: Result<TestData, _> = resp_invalid_json.json();
        assert!(parsed_invalid.is_err());
        assert!(matches!(parsed_invalid.unwrap_err(), HttpError::Parse(_)));
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

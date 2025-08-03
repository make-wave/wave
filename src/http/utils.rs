use crate::http::error::HttpError;
use ::http::Method;

/// Parse a string into an HTTP method
///
/// Supports the standard HTTP methods commonly used in REST APIs and web development.
/// Each method has specific semantics according to HTTP specifications.
///
/// # Examples
///
/// ```
/// use wave::http::parse_method;
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

#[cfg(test)]
mod tests {
    use super::*;

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
}


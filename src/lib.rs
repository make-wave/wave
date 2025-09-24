pub mod collection;
pub mod error;
pub mod http;
pub mod printer;

use crate::http::{Client, HttpRequest, RequestBody, ReqwestBackend};
use ::http::{HeaderMap, Method};
use clap::{Parser, Subcommand};
use error::{CliError, CollectionError, WaveError};
use std::collections::HashMap;

// Type aliases for clarity and consistency
pub type KeyValuePairs = Vec<(String, String)>;
pub type Headers = KeyValuePairs;
pub type FormData = KeyValuePairs;

// Re-export http_client types for backward compatibility
pub mod http_client {
    pub use crate::http::*;
}

/// Convert Vec of header tuples to HeaderMap
fn headers_to_map(headers: Headers) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for (key, value) in headers {
        if let (Ok(header_name), Ok(header_value)) = (
            key.parse::<::http::HeaderName>(),
            value.parse::<::http::HeaderValue>(),
        ) {
            header_map.insert(header_name, header_value);
        }
    }
    header_map
}

#[derive(Subcommand)]
pub enum Command {
    /// Send a GET request
    Get {
        /// The URL to send the request to
        url: String,
        /// Headers and body data (key:value or key=value)
        #[arg(value_parser, trailing_var_arg = true)]
        params: Vec<String>,
        /// Print the full response (status, headers, body)
        #[arg(short, long)]
        verbose: bool,
    },
    /// Send a POST request
    Post {
        url: String,
        #[arg(value_parser, trailing_var_arg = true)]
        params: Vec<String>,
        #[arg(long)]
        form: bool,
        #[arg(short, long)]
        verbose: bool,
    },
    /// Send a PUT request
    Put {
        url: String,
        #[arg(value_parser, trailing_var_arg = true)]
        params: Vec<String>,
        #[arg(long)]
        form: bool,
        #[arg(short, long)]
        verbose: bool,
    },
    /// Send a PATCH request
    Patch {
        url: String,
        #[arg(value_parser, trailing_var_arg = true)]
        params: Vec<String>,
        #[arg(long)]
        form: bool,
        #[arg(short, long)]
        verbose: bool,
    },
    /// Send a DELETE request
    Delete {
        url: String,
        #[arg(value_parser, trailing_var_arg = true)]
        params: Vec<String>,
        #[arg(short, long)]
        verbose: bool,
    },
    /// Run a saved request from a collection
    #[command(
        short_flag = 'c',
        visible_alias = "c",
        visible_short_flag_alias = 'c',
        about = "Run a saved request from a collection"
    )]
    Collection {
        /// Name of the collection
        collection: String,
        /// Name of the request in the collection
        request: String,
        #[arg(short, long)]
        verbose: bool,
        /// Headers and body data (key:value or key=value)
        #[arg(value_parser, trailing_var_arg = true)]
        params: Vec<String>,
    },
}

#[derive(Parser)]
#[command(name = "wave")]
#[command(author, version, about, long_about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

pub type HeaderDataTuple = (Headers, FormData);

pub fn parse_params(params: &[String]) -> HeaderDataTuple {
    let mut headers = Vec::new();
    let mut data = Vec::new();
    for param in params {
        // Ignore --form if present in params
        if param == "--form" {
            continue;
        }
        if let Some((k, v)) = param.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        } else if let Some((k, v)) = param.split_once('=') {
            data.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    (headers, data)
}

/// Validates and parses parameters, returning errors for invalid formats
pub fn validate_params(params: &[String]) -> Result<HeaderDataTuple, WaveError> {
    let mut headers = Vec::new();
    let mut data = Vec::new();

    for param in params {
        // Ignore --form if present in params
        if param == "--form" {
            continue;
        }

        if let Some((k, v)) = param.split_once(':') {
            let key = k.trim();
            let value = v.trim();

            // Validate header format
            if key.is_empty() {
                return Err(WaveError::Cli(CliError::InvalidHeaderFormat(param.clone())));
            }
            if key.contains(' ') {
                return Err(WaveError::Cli(CliError::InvalidHeaderFormat(param.clone())));
            }

            headers.push((key.to_string(), value.to_string()));
        } else if let Some((k, v)) = param.split_once('=') {
            let key = k.trim();
            let value = v.trim();

            // Validate body data format
            if key.is_empty() {
                return Err(WaveError::Cli(CliError::InvalidBodyFormat(param.clone())));
            }

            data.push((key.to_string(), value.to_string()));
        } else {
            // Parameter doesn't match either format
            return Err(WaveError::Cli(CliError::InvalidHeaderFormat(format!(
                "Parameter '{param}' must be in 'key:value' (header) or 'key=value' (body) format"
            ))));
        }
    }

    Ok((headers, data))
}

/// Validates URL format
pub fn validate_url(url: &str) -> Result<String, WaveError> {
    if url.trim().is_empty() {
        return Err(WaveError::Cli(CliError::InvalidUrl(
            "URL cannot be empty".to_string(),
        )));
    }

    // Add scheme if missing
    let url_with_scheme = if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("http://{url}")
    };

    // Basic URL validation
    if !url_with_scheme.contains('.') {
        return Err(WaveError::Cli(CliError::InvalidUrl(url.to_string())));
    }

    Ok(url_with_scheme)
}

pub fn ensure_url_scheme(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("http://{url}")
    }
}

use indicatif::{ProgressBar, ProgressStyle};
use printer::print_response;
use std::time::Duration;

pub async fn run_with_spinner<F, Fut, T>(message: &str, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let pb = ProgressBar::new_spinner();
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));

    // Try to set a fancy template, fall back to simple spinner if it fails
    let style_result = ProgressStyle::default_spinner()
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
        .template("{spinner} {msg}");

    match style_result {
        Ok(style) => pb.set_style(style),
        Err(_) => {
            // Fallback to basic spinner without template
            pb.set_style(ProgressStyle::default_spinner());
        }
    }

    let result = f().await;
    pb.finish_and_clear();
    result
}

pub async fn execute_request_with_spinner(
    req: &HttpRequest,
    spinner_msg: &str,
    verbose: bool,
) -> Result<(), WaveError> {
    let client = Client::new(ReqwestBackend);
    let result = run_with_spinner(spinner_msg, || client.send(req)).await;
    print_response(result, verbose);
    Ok(())
}

pub async fn handle_get(
    url: &str,
    params: &[String],
    verbose: bool,
    spinner_msg: &str,
) -> Result<(), WaveError> {
    let url = validate_url(url)?;
    let (headers, _) = validate_params(params)?;
    let req = HttpRequest::new(&url, Method::GET, None, headers_to_map(headers));
    execute_request_with_spinner(&req, spinner_msg, verbose).await
}

pub async fn handle_method_with_body(
    method: Method,
    url: &str,
    params: &[String],
    form: bool,
    verbose: bool,
    spinner_msg: &str,
) -> Result<(), WaveError> {
    let url = validate_url(url)?;
    let (headers, data) = validate_params(params)?;

    let req = if form {
        HttpRequest::builder(&url, method)
            .headers(headers_to_map(headers))
            .body(RequestBody::form(data))
            .build()
    } else {
        match RequestBody::json(&data.into_iter().collect::<HashMap<String, String>>()) {
            Ok(body) => HttpRequest::builder(&url, method)
                .headers(headers_to_map(headers))
                .body(body)
                .build(),
            Err(_) => HttpRequest::new(
                &url,
                method,
                Some("{}".to_string()),
                headers_to_map(headers),
            ),
        }
    };

    execute_request_with_spinner(&req, spinner_msg, verbose).await
}

pub async fn handle_post(
    url: &str,
    params: &[String],
    form: bool,
    verbose: bool,
    spinner_msg: &str,
) -> Result<(), WaveError> {
    handle_method_with_body(Method::POST, url, params, form, verbose, spinner_msg).await
}

pub async fn handle_put(
    url: &str,
    params: &[String],
    form: bool,
    verbose: bool,
    spinner_msg: &str,
) -> Result<(), WaveError> {
    handle_method_with_body(Method::PUT, url, params, form, verbose, spinner_msg).await
}

pub async fn handle_patch(
    url: &str,
    params: &[String],
    form: bool,
    verbose: bool,
    spinner_msg: &str,
) -> Result<(), WaveError> {
    handle_method_with_body(Method::PATCH, url, params, form, verbose, spinner_msg).await
}

pub async fn handle_delete(
    url: &str,
    params: &[String],
    verbose: bool,
    spinner_msg: &str,
) -> Result<(), WaveError> {
    let url = validate_url(url)?;
    let (headers, _) = validate_params(params)?;
    let req = HttpRequest::new(&url, Method::DELETE, None, headers_to_map(headers));
    execute_request_with_spinner(&req, spinner_msg, verbose).await
}

/// Parse a CLI parameter value to appropriate JSON type
fn parse_cli_value_to_json(value: &str) -> serde_json::Value {
    // Try parsing as integer first
    if let Ok(int_val) = value.parse::<i64>() {
        return serde_json::Value::Number(int_val.into());
    }
    // Try parsing as float
    if let Ok(float_val) = value.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(float_val) {
            return serde_json::Value::Number(num);
        }
    }
    // Try parsing as boolean
    if let Ok(bool_val) = value.parse::<bool>() {
        return serde_json::Value::Bool(bool_val);
    }
    // Default to string
    serde_json::Value::String(value.to_string())
}

/// Merge collection JSON with CLI parameters, preserving types from collection
fn merge_json_with_cli_params(
    collection_json: Option<serde_json::Value>,
    cli_body: &[(String, String)],
) -> serde_json::Value {
    let mut result = collection_json.unwrap_or(serde_json::json!({}));

    if let Some(obj) = result.as_object_mut() {
        for (key, value) in cli_body {
            // CLI parameters override collection values, with type inference
            let json_value = parse_cli_value_to_json(value);
            obj.insert(key.clone(), json_value);
        }
    } else if !cli_body.is_empty() {
        // If collection doesn't have JSON body but CLI has params, create new object
        let mut obj = serde_json::Map::new();
        for (key, value) in cli_body {
            let json_value = parse_cli_value_to_json(value);
            obj.insert(key.clone(), json_value);
        }
        result = serde_json::Value::Object(obj);
    }

    result
}

/// Merge headers and body data, with CLI params overriding collection params
fn merge_headers_and_body(
    collection_headers: &[(String, String)],
    collection_body: &[(String, String)],
    cli_headers: &[(String, String)],
    cli_body: &[(String, String)],
) -> (Headers, FormData) {
    let mut headers = collection_headers.to_vec();
    let mut body = collection_body.to_vec();

    // Override headers with CLI values
    for (cli_key, cli_value) in cli_headers {
        if let Some(pos) = headers.iter().position(|(k, _)| k == cli_key) {
            headers[pos].1 = cli_value.clone();
        } else {
            headers.push((cli_key.clone(), cli_value.clone()));
        }
    }

    // Override body with CLI values
    for (cli_key, cli_value) in cli_body {
        if let Some(pos) = body.iter().position(|(k, _)| k == cli_key) {
            body[pos].1 = cli_value.clone();
        } else {
            body.push((cli_key.clone(), cli_value.clone()));
        }
    }

    (headers, body)
}

/// Parse form data string to key-value pairs
fn parse_form_to_key_value_pairs(form_str: &str) -> KeyValuePairs {
    form_str
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let k = parts.next()?;
            let v = parts.next().unwrap_or("");
            Some((k.to_string(), v.to_string()))
        })
        .collect()
}

// Collection request handling
fn prepare_collection_headers_and_body(
    resolved: &collection::Request,
) -> (Headers, Option<serde_json::Value>, bool) {
    let mut headers: Headers = resolved
        .headers
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();
    match &resolved.body {
        Some(collection::Body::Json(map)) => {
            let json_obj = serde_json::Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), collection::yaml_to_json(v)))
                    .collect(),
            );
            if !headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            {
                headers.push(("Content-Type".to_string(), "application/json".to_string()));
            }
            (headers, Some(json_obj), false)
        }
        Some(collection::Body::Form(map)) => {
            let form_data: FormData = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            let mut header_map = ::http::HeaderMap::new();
            let body = http::RequestBody::form(form_data);
            let form_str = body.serialize(&mut header_map);

            // Convert HeaderMap back to Vec for compatibility
            let form_headers: Headers = header_map
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            headers.extend(form_headers);
            // For form data, we return the serialized string as a JSON string value
            (headers, Some(serde_json::Value::String(form_str)), true)
        }
        None => (headers, None, false),
    }
}

pub async fn handle_collection(
    collection_name: &str,
    request_name: &str,
    verbose: bool,
    params: &[String],
) -> Result<(), WaveError> {
    let yaml_path = format!(".wave/{collection_name}.yaml");
    let yml_path = format!(".wave/{collection_name}.yml");
    let coll_result =
        collection::load_collection(&yaml_path).or_else(|_| collection::load_collection(&yml_path));

    match coll_result {
        Ok(coll) => {
            let file_vars = coll.variables.unwrap_or_default();
            match coll.requests.iter().find(|r| r.name == request_name) {
                Some(req) => match collection::resolve_request_vars(req, &file_vars) {
                    Ok(resolved) => {
                        let spinner_msg = format!("{} {}", resolved.method, resolved.url);
                        // Parse CLI params for potential override
                        let (cli_headers, cli_body) = parse_params(params);
                        match resolved.method {
                            Method::GET => {
                                let collection_headers: Headers =
                                    resolved.headers.unwrap_or_default().into_iter().collect();
                                let (headers, _) = merge_headers_and_body(
                                    &collection_headers,
                                    &[],
                                    &cli_headers,
                                    &[],
                                );
                                let req = HttpRequest::new(
                                    &resolved.url,
                                    Method::GET,
                                    None,
                                    headers_to_map(headers),
                                );
                                execute_request_with_spinner(&req, &spinner_msg, verbose).await?;
                            }
                            Method::DELETE => {
                                let collection_headers: Headers =
                                    resolved.headers.unwrap_or_default().into_iter().collect();
                                let (headers, _) = merge_headers_and_body(
                                    &collection_headers,
                                    &[],
                                    &cli_headers,
                                    &[],
                                );
                                let req = HttpRequest::new(
                                    &resolved.url,
                                    Method::DELETE,
                                    None,
                                    headers_to_map(headers),
                                );
                                execute_request_with_spinner(&req, &spinner_msg, verbose).await?;
                            }
                            Method::POST | Method::PUT | Method::PATCH => {
                                let (collection_headers, collection_json, is_form) =
                                    prepare_collection_headers_and_body(&resolved);

                                // Merge headers (CLI overrides collection)
                                let (merged_headers, _) = merge_headers_and_body(
                                    &collection_headers,
                                    &[],
                                    &cli_headers,
                                    &[],
                                );

                                // Handle body based on type
                                let final_body = if is_form {
                                    // For form data, extract the string from JSON and merge with CLI params
                                    let form_str = collection_json
                                        .as_ref()
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let collection_body_data = if form_str.is_empty() {
                                        vec![]
                                    } else {
                                        parse_form_to_key_value_pairs(&form_str)
                                    };
                                    let (_, merged_body_data) = merge_headers_and_body(
                                        &[],
                                        &collection_body_data,
                                        &[],
                                        &cli_body,
                                    );
                                    merged_body_data
                                        .iter()
                                        .map(|(k, v)| format!("{k}={v}"))
                                        .collect::<Vec<_>>()
                                        .join("&")
                                } else {
                                    // JSON encoding - use new merge function that preserves types
                                    let merged_json =
                                        merge_json_with_cli_params(collection_json, &cli_body);
                                    serde_json::to_string(&merged_json)
                                        .unwrap_or_else(|_| "{}".to_string())
                                };

                                let req = HttpRequest::new(
                                    &resolved.url,
                                    resolved.method.clone(),
                                    Some(final_body),
                                    headers_to_map(merged_headers),
                                );
                                execute_request_with_spinner(&req, &spinner_msg, verbose).await?;
                            }
                            _ => {
                                return Err(WaveError::Cli(CliError::UnsupportedMethod(
                                    resolved.method.to_string(),
                                )))
                            }
                        }
                    }
                    Err(e) => {
                        return Err(WaveError::Collection(CollectionError::VariableResolution(
                            e.to_string(),
                        )))
                    }
                },
                None => {
                    return Err(WaveError::Collection(CollectionError::RequestNotFound {
                        collection: collection_name.to_string(),
                        request: request_name.to_string(),
                    }));
                }
            }
        }
        Err(_e) => {
            println!("{_e}");
            return Err(WaveError::Collection(CollectionError::FileNotFound(
                format!("{collection_name}.yaml or {collection_name}.yml"),
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use core::f64;

    use super::*;

    #[test]
    fn test_parse_params_json_body() {
        let params = vec![
            "name=joe".to_string(),
            "age=42".to_string(),
            "Authorization:Bearer123".to_string(),
        ];
        let (headers, data) = parse_params(&params);
        assert_eq!(
            headers,
            vec![("Authorization".to_string(), "Bearer123".to_string())]
        );
        assert_eq!(
            data,
            vec![
                ("name".to_string(), "joe".to_string()),
                ("age".to_string(), "42".to_string())
            ]
        );
    }

    #[test]
    fn test_parse_params_form_flag_ignored() {
        let params = vec![
            "--form".to_string(),
            "foo=bar".to_string(),
            "baz=qux".to_string(),
            "X-Test:1".to_string(),
        ];
        let (headers, data) = parse_params(&params);
        assert_eq!(headers, vec![("X-Test".to_string(), "1".to_string())]);
        assert_eq!(
            data,
            vec![
                ("foo".to_string(), "bar".to_string()),
                ("baz".to_string(), "qux".to_string())
            ]
        );
    }

    #[test]
    fn test_validate_url_with_scheme() {
        assert_eq!(
            validate_url("https://example.com").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            validate_url("http://example.com").unwrap(),
            "http://example.com"
        );
    }

    #[test]
    fn test_validate_url_adds_scheme() {
        assert_eq!(validate_url("example.com").unwrap(), "http://example.com");
        assert_eq!(
            validate_url("api.example.com").unwrap(),
            "http://api.example.com"
        );
    }

    #[test]
    fn test_validate_url_rejects_empty() {
        assert!(validate_url("").is_err());
        assert!(validate_url("   ").is_err());
    }

    #[test]
    fn test_validate_url_rejects_invalid() {
        assert!(validate_url("localhost").is_err()); // No dot
        assert!(validate_url("not-a-url").is_err()); // No dot
    }

    #[test]
    fn test_validate_params_valid() {
        let params = vec![
            "Authorization:Bearer123".to_string(),
            "name=joe".to_string(),
            "age=42".to_string(),
        ];
        let result = validate_params(&params).unwrap();
        assert_eq!(
            result.0,
            vec![("Authorization".to_string(), "Bearer123".to_string())]
        );
        assert_eq!(
            result.1,
            vec![
                ("name".to_string(), "joe".to_string()),
                ("age".to_string(), "42".to_string())
            ]
        );
    }

    #[test]
    fn test_validate_params_empty_header_key() {
        let params = vec![":Bearer123".to_string()];
        assert!(validate_params(&params).is_err());
    }

    #[test]
    fn test_validate_params_header_with_space() {
        let params = vec!["Auth orization:Bearer123".to_string()];
        assert!(validate_params(&params).is_err());
    }

    #[test]
    fn test_validate_params_empty_body_key() {
        let params = vec!["=value".to_string()];
        assert!(validate_params(&params).is_err());
    }

    #[test]
    fn test_validate_params_invalid_format() {
        let params = vec!["invalid-param".to_string()];
        assert!(validate_params(&params).is_err());
    }

    #[tokio::test]
    async fn test_error_propagation_integration() {
        // Test that validation errors propagate through the handle functions
        let result = handle_get("", &[], false, "test").await;
        assert!(result.is_err());

        let result = handle_get("localhost", &["invalid-param".to_string()], false, "test").await;
        assert!(result.is_err());

        let result = handle_get("example.com", &[":empty-key".to_string()], false, "test").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_edge_cases() {
        // Test various URL edge cases
        assert!(validate_url("https://").is_err());
        assert!(validate_url("http://").is_err());
        assert!(validate_url("ftp://example.com").is_ok()); // We allow any protocol and add http if missing
        assert!(validate_url("localhost:8080").is_err()); // No dot
        assert!(validate_url("192.168.1.1").is_ok()); // IP addresses have dots
    }

    #[test]
    fn test_validate_params_edge_cases() {
        // Empty values should be allowed
        assert!(validate_params(&["key:".to_string()]).is_ok());
        assert!(validate_params(&["key=".to_string()]).is_ok());

        // Special characters in values should be allowed
        assert!(validate_params(&["Authorization:Bearer token=with=equals".to_string()]).is_ok());
        assert!(validate_params(&["data=value:with:colons".to_string()]).is_ok());

        // Multiple equals/colons - only first one is used as separator
        let result = validate_params(&["key=value=more".to_string()]).unwrap();
        assert_eq!(result.1[0].1, "value=more");

        let result = validate_params(&["key:value:more".to_string()]).unwrap();
        assert_eq!(result.0[0].1, "value:more");
    }

    #[test]
    fn test_merge_headers_and_body() {
        let collection_headers = vec![
            ("Authorization".to_string(), "Bearer123".to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
        ];
        let collection_body = vec![
            ("name".to_string(), "collection".to_string()),
            ("type".to_string(), "test".to_string()),
        ];
        let cli_headers = vec![
            ("Authorization".to_string(), "BearerCLI".to_string()),
            ("X-Custom".to_string(), "header".to_string()),
        ];
        let cli_body = vec![
            ("name".to_string(), "override".to_string()),
            ("new_field".to_string(), "value".to_string()),
        ];

        let (merged_headers, merged_body) = merge_headers_and_body(
            &collection_headers,
            &collection_body,
            &cli_headers,
            &cli_body,
        );

        // Check that CLI overrides collection headers
        assert!(merged_headers.contains(&("Authorization".to_string(), "BearerCLI".to_string())));
        // Check that collection headers are preserved when not overridden
        assert!(
            merged_headers.contains(&("Content-Type".to_string(), "application/json".to_string()))
        );
        // Check that new CLI headers are added
        assert!(merged_headers.contains(&("X-Custom".to_string(), "header".to_string())));

        // Check that CLI overrides collection body
        assert!(merged_body.contains(&("name".to_string(), "override".to_string())));
        // Check that collection body is preserved when not overridden
        assert!(merged_body.contains(&("type".to_string(), "test".to_string())));
        // Check that new CLI body fields are added
        assert!(merged_body.contains(&("new_field".to_string(), "value".to_string())));
    }

    #[test]
    fn test_parse_cli_value_to_json() {
        // Test integer parsing
        assert_eq!(
            parse_cli_value_to_json("42"),
            serde_json::Value::Number(42.into())
        );
        assert_eq!(
            parse_cli_value_to_json("-123"),
            serde_json::Value::Number((-123).into())
        );

        // Test float parsing
        if let serde_json::Value::Number(n) = parse_cli_value_to_json("2.5") {
            assert_eq!(n.as_f64(), Some(2.5));
        } else {
            panic!("Expected number value for float");
        }

        // Test boolean parsing
        assert_eq!(
            parse_cli_value_to_json("true"),
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            parse_cli_value_to_json("false"),
            serde_json::Value::Bool(false)
        );

        // Test string fallback
        assert_eq!(
            parse_cli_value_to_json("hello"),
            serde_json::Value::String("hello".to_string())
        );
        assert_eq!(
            parse_cli_value_to_json("123abc"),
            serde_json::Value::String("123abc".to_string())
        );
        assert_eq!(
            parse_cli_value_to_json(""),
            serde_json::Value::String("".to_string())
        );
    }

    #[test]
    fn test_merge_json_with_cli_params_empty_collection() {
        // Test with no collection JSON
        let cli_params = vec![
            ("name".to_string(), "alice".to_string()),
            ("age".to_string(), "30".to_string()),
            ("active".to_string(), "true".to_string()),
        ];

        let result = merge_json_with_cli_params(None, &cli_params);

        let expected = serde_json::json!({
            "name": "alice",
            "age": 30,
            "active": true
        });

        assert_eq!(result, expected);
    }

    #[test]
    fn test_merge_json_with_cli_params_preserve_types() {
        // Create collection JSON with various types
        let collection_json = serde_json::json!({
            "user_id": 42,
            "score": 98.5,
            "active": true,
            "name": "original",
            "metadata": {
                "created": "2023-01-01"
            }
        });

        // CLI params that should override some values
        let cli_params = vec![
            ("name".to_string(), "updated".to_string()),
            ("new_field".to_string(), "123".to_string()),
        ];

        let result = merge_json_with_cli_params(Some(collection_json), &cli_params);

        let expected = serde_json::json!({
            "user_id": 42,           // Preserved from collection
            "score": 98.5,           // Preserved from collection
            "active": true,          // Preserved from collection
            "name": "updated",       // Overridden by CLI (as string)
            "metadata": {            // Preserved from collection
                "created": "2023-01-01"
            },
            "new_field": 123         // Added from CLI (parsed as number)
        });

        assert_eq!(result, expected);
    }

    #[test]
    fn test_merge_json_with_cli_params_no_cli_params() {
        // Test that collection JSON is preserved when no CLI params
        let collection_json = serde_json::json!({
            "count": 42,
            "rate": f64::consts::PI,
            "enabled": false
        });

        let result = merge_json_with_cli_params(Some(collection_json.clone()), &[]);

        assert_eq!(result, collection_json);
    }

    #[test]
    fn test_merge_json_with_cli_params_type_inference() {
        // Test that CLI parameters are properly typed
        let collection_json = serde_json::json!({});

        let cli_params = vec![
            ("integer".to_string(), "42".to_string()),
            ("negative".to_string(), "-10".to_string()),
            ("float".to_string(), "2.5".to_string()),
            ("bool_true".to_string(), "true".to_string()),
            ("bool_false".to_string(), "false".to_string()),
            ("string".to_string(), "hello world".to_string()),
            ("number_like_string".to_string(), "123abc".to_string()),
        ];

        let result = merge_json_with_cli_params(Some(collection_json), &cli_params);

        assert_eq!(result["integer"], serde_json::Value::Number(42.into()));
        assert_eq!(result["negative"], serde_json::Value::Number((-10).into()));
        if let serde_json::Value::Number(n) = &result["float"] {
            assert_eq!(n.as_f64(), Some(2.5));
        } else {
            panic!("Expected number value for float");
        }
        assert_eq!(result["bool_true"], serde_json::Value::Bool(true));
        assert_eq!(result["bool_false"], serde_json::Value::Bool(false));
        assert_eq!(
            result["string"],
            serde_json::Value::String("hello world".to_string())
        );
        assert_eq!(
            result["number_like_string"],
            serde_json::Value::String("123abc".to_string())
        );
    }
}

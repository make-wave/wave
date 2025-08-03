pub mod collection;
pub mod error;
pub mod http_client;
pub mod printer;

use clap::{Parser, Subcommand};
use error::{WaveError, CliError, CollectionError};
use http_client::{Client, HttpMethod, HttpRequest, RequestBody, ReqwestBackend};
use std::collections::HashMap;

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
    },
}

#[derive(Parser)]
#[command(name = "wave")]
#[command(author, version, about, long_about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

pub type HeaderDataTuple = (Vec<(String, String)>, Vec<(String, String)>);

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
        return Err(WaveError::Cli(CliError::InvalidUrl("URL cannot be empty".to_string())));
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

// New: Command logic helpers
use indicatif::{ProgressBar, ProgressStyle};
use printer::print_response;
use std::time::Duration;

pub fn run_with_spinner<F, T>(message: &str, f: F) -> T
where
    F: FnOnce() -> T,
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
    
    let result = f();
    pb.finish_and_clear();
    result
}

// New: Common HTTP execution logic
pub fn execute_request_with_spinner(req: &HttpRequest, spinner_msg: &str, verbose: bool) -> Result<(), WaveError> {
    let client = Client::new(ReqwestBackend);
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| WaveError::Runtime(format!("Failed to create async runtime: {e}")))?;

    let result = run_with_spinner(spinner_msg, || rt.block_on(client.send(req)));
    print_response(result, verbose);
    Ok(())
}

pub fn handle_get(url: &str, params: &[String], verbose: bool, spinner_msg: &str) -> Result<(), WaveError> {
    let url = validate_url(url)?;
    let (headers, _) = validate_params(params)?;
    let req = HttpRequest::new_with_headers(&url, HttpMethod::Get, None, headers);
    execute_request_with_spinner(&req, spinner_msg, verbose)
}

// Consolidated handler for POST/PUT/PATCH methods with body data
pub fn handle_method_with_body(
    method: HttpMethod,
    url: &str,
    params: &[String],
    form: bool,
    verbose: bool,
    spinner_msg: &str,
) -> Result<(), WaveError> {
    let url = validate_url(url)?;
    let (headers, data) = validate_params(params)?;

    let req = if form {
        let body = RequestBody::form(data);
        HttpRequest::with_body_from_headers(&url, method, Some(body), headers)
    } else {
        match RequestBody::json(&data.into_iter().collect::<HashMap<String, String>>()) {
            Ok(body) => HttpRequest::with_body_from_headers(&url, method, Some(body), headers),
            Err(_) => HttpRequest::new_with_headers(&url, method, Some("{}".to_string()), headers),
        }
    };

    execute_request_with_spinner(&req, spinner_msg, verbose)
}

pub fn handle_post(url: &str, params: &[String], form: bool, verbose: bool, spinner_msg: &str) -> Result<(), WaveError> {
    handle_method_with_body(HttpMethod::Post, url, params, form, verbose, spinner_msg)
}

pub fn handle_put(url: &str, params: &[String], form: bool, verbose: bool, spinner_msg: &str) -> Result<(), WaveError> {
    handle_method_with_body(HttpMethod::Put, url, params, form, verbose, spinner_msg)
}

pub fn handle_patch(url: &str, params: &[String], form: bool, verbose: bool, spinner_msg: &str) -> Result<(), WaveError> {
    handle_method_with_body(HttpMethod::Patch, url, params, form, verbose, spinner_msg)
}

pub fn handle_delete(url: &str, params: &[String], verbose: bool, spinner_msg: &str) -> Result<(), WaveError> {
    let url = validate_url(url)?;
    let (headers, _) = validate_params(params)?;
    let req = HttpRequest::new_with_headers(&url, HttpMethod::Delete, None, headers);
    execute_request_with_spinner(&req, spinner_msg, verbose)
}

// Collection request handling
fn prepare_collection_headers_and_body(
    resolved: &collection::Request,
) -> (Vec<(String, String)>, String, bool) {
    let mut headers: Vec<(String, String)> = resolved
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
            let json_str = serde_json::to_string(&json_obj).unwrap_or_else(|_| "{}".to_string());
            (headers, json_str, false)
        }
        Some(collection::Body::Form(map)) => {
            let mut header_map = http::HeaderMap::new();
            let form_str = Client::<ReqwestBackend>::prepare_form_body(
                &map.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                &mut header_map,
            );
            // Convert HeaderMap back to Vec for compatibility
            let form_headers: Vec<(String, String)> = header_map
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            headers.extend(form_headers);
            (headers, form_str, true)
        }
        None => (headers, "".to_string(), false),
    }
}

pub fn handle_collection(collection_name: &str, request_name: &str, verbose: bool) -> Result<(), WaveError> {
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
                        match resolved.method {
                            HttpMethod::Get => {
                                let headers: Vec<(String, String)> =
                                    resolved.headers.unwrap_or_default().into_iter().collect();
                                let req = HttpRequest::new_with_headers(
                                    &resolved.url,
                                    HttpMethod::Get,
                                    None,
                                    headers,
                                );
                                execute_request_with_spinner(&req, &spinner_msg, verbose)?;
                            }
                            HttpMethod::Delete => {
                                let headers: Vec<(String, String)> =
                                    resolved.headers.unwrap_or_default().into_iter().collect();
                                let req = HttpRequest::new_with_headers(
                                    &resolved.url,
                                    HttpMethod::Delete,
                                    None,
                                    headers,
                                );
                                execute_request_with_spinner(&req, &spinner_msg, verbose)?;
                            }
                            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => {
                                let (headers, body, _is_form) =
                                    prepare_collection_headers_and_body(&resolved);
                                let req = HttpRequest::new_with_headers(
                                    &resolved.url,
                                    resolved.method.clone(),
                                    Some(body),
                                    headers,
                                );
                                execute_request_with_spinner(&req, &spinner_msg, verbose)?;
                            }
                            _ => return Err(WaveError::Cli(CliError::UnsupportedMethod(resolved.method.to_string()))),
                        }
                    }
                    Err(e) => return Err(WaveError::Collection(CollectionError::VariableResolution(e.to_string()))),
                },
                None => {
                    return Err(WaveError::Collection(CollectionError::RequestNotFound {
                        collection: collection_name.to_string(),
                        request: request_name.to_string(),
                    }));
                }
            }
        }
        Err(_e) => return Err(WaveError::Collection(CollectionError::FileNotFound(format!("{collection_name}.yaml or {collection_name}.yml")))),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
        assert_eq!(validate_url("https://example.com").unwrap(), "https://example.com");
        assert_eq!(validate_url("http://example.com").unwrap(), "http://example.com");
    }

    #[test]
    fn test_validate_url_adds_scheme() {
        assert_eq!(validate_url("example.com").unwrap(), "http://example.com");
        assert_eq!(validate_url("api.example.com").unwrap(), "http://api.example.com");
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
        assert_eq!(result.0, vec![("Authorization".to_string(), "Bearer123".to_string())]);
        assert_eq!(result.1, vec![
            ("name".to_string(), "joe".to_string()),
            ("age".to_string(), "42".to_string())
        ]);
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

    #[test]
    fn test_error_propagation_integration() {
        // Test that validation errors propagate through the handle functions
        let result = handle_get("", &[], false, "test");
        assert!(result.is_err());
        
        let result = handle_get("localhost", &["invalid-param".to_string()], false, "test");
        assert!(result.is_err());
        
        let result = handle_get("example.com", &[":empty-key".to_string()], false, "test");
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
}

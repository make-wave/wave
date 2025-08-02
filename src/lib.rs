pub mod collection;
pub mod http_client;
pub mod printer;

use clap::{Parser, Subcommand};
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
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} {msg}")
            .unwrap(),
    );
    let result = f();
    pb.finish_and_clear();
    result
}

// New: Common HTTP execution logic
pub fn execute_request_with_spinner(req: &HttpRequest, spinner_msg: &str, verbose: bool) {
    let client = Client::new(ReqwestBackend);
    let rt = tokio::runtime::Runtime::new().unwrap();

    let result = run_with_spinner(spinner_msg, || rt.block_on(client.send(req)));
    print_response(result, verbose);
}

pub fn handle_get(url: &str, params: &[String], verbose: bool, spinner_msg: &str) {
    let url = ensure_url_scheme(url);
    let (headers, _) = parse_params(params);
    let req = HttpRequest::new_with_headers(&url, HttpMethod::Get, None, headers);
    execute_request_with_spinner(&req, spinner_msg, verbose);
}

// Consolidated handler for POST/PUT/PATCH methods with body data
pub fn handle_method_with_body(
    method: HttpMethod,
    url: &str,
    params: &[String],
    form: bool,
    verbose: bool,
    spinner_msg: &str,
) {
    let url = ensure_url_scheme(url);
    let (headers, data) = parse_params(params);

    let req = if form {
        let body = RequestBody::form(data);
        HttpRequest::with_body_from_headers(&url, method, Some(body), headers)
    } else {
        match RequestBody::json(&data.into_iter().collect::<HashMap<String, String>>()) {
            Ok(body) => HttpRequest::with_body_from_headers(&url, method, Some(body), headers),
            Err(_) => HttpRequest::new_with_headers(&url, method, Some("{}".to_string()), headers),
        }
    };

    execute_request_with_spinner(&req, spinner_msg, verbose);
}

pub fn handle_post(url: &str, params: &[String], form: bool, verbose: bool, spinner_msg: &str) {
    handle_method_with_body(HttpMethod::Post, url, params, form, verbose, spinner_msg);
}

pub fn handle_put(url: &str, params: &[String], form: bool, verbose: bool, spinner_msg: &str) {
    handle_method_with_body(HttpMethod::Put, url, params, form, verbose, spinner_msg);
}

pub fn handle_patch(url: &str, params: &[String], form: bool, verbose: bool, spinner_msg: &str) {
    handle_method_with_body(HttpMethod::Patch, url, params, form, verbose, spinner_msg);
}

pub fn handle_delete(url: &str, params: &[String], verbose: bool, spinner_msg: &str) {
    let url = ensure_url_scheme(url);
    let (headers, _) = parse_params(params);
    let req = HttpRequest::new_with_headers(&url, HttpMethod::Delete, None, headers);
    execute_request_with_spinner(&req, spinner_msg, verbose);
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

pub fn handle_collection(collection_name: &str, request_name: &str, verbose: bool) {
    let yaml_path = format!(".wave/{collection_name}.yaml");
    let yml_path = format!(".wave/{collection_name}.yml");
    let coll_result = collection::load_collection(&yaml_path).or_else(|_| collection::load_collection(&yml_path));
    
    match coll_result {
        Ok(coll) => {
            let file_vars = coll.variables.unwrap_or_default();
            match coll.requests.iter().find(|r| r.name == request_name) {
                Some(req) => match collection::resolve_request_vars(req, &file_vars) {
                    Ok(resolved) => {
                        let spinner_msg = format!("{} {}", resolved.method, resolved.url);
                        match resolved.method {
                            HttpMethod::Get => {
                                let headers: Vec<(String, String)> = resolved
                                    .headers
                                    .unwrap_or_default()
                                    .into_iter()
                                    .collect();
                                let req = HttpRequest::new_with_headers(
                                    &resolved.url,
                                    HttpMethod::Get,
                                    None,
                                    headers,
                                );
                                execute_request_with_spinner(&req, &spinner_msg, verbose);
                            }
                            HttpMethod::Delete => {
                                let headers: Vec<(String, String)> = resolved
                                    .headers
                                    .unwrap_or_default()
                                    .into_iter()
                                    .collect();
                                let req = HttpRequest::new_with_headers(
                                    &resolved.url,
                                    HttpMethod::Delete,
                                    None,
                                    headers,
                                );
                                execute_request_with_spinner(&req, &spinner_msg, verbose);
                            }
                            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch => {
                                let (headers, body, _is_form) = prepare_collection_headers_and_body(&resolved);
                                let req = HttpRequest::new_with_headers(
                                    &resolved.url,
                                    resolved.method.clone(),
                                    Some(body),
                                    headers,
                                );
                                execute_request_with_spinner(&req, &spinner_msg, verbose);
                            }
                            _ => eprintln!("Unsupported method: {}", resolved.method),
                        }
                    }
                    Err(e) => eprintln!("Variable resolution error: {e}"),
                },
                None => {
                    eprintln!("Request '{request_name}' not found in collection '{collection_name}'.");
                }
            }
        }
        Err(e) => eprintln!("Failed to load collection '{collection_name}': {e}"),
    }
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
}

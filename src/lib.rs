pub mod collection;
pub mod http_client;
pub mod printer;

use clap::Parser;
use http_client::{Client, ReqwestBackend};

#[derive(Parser)]
#[command(name = "wave")]
#[command(about = "A terminal-based HTTP client", long_about = None)]
pub struct Cli {
    /// HTTP method or collection name
    #[arg()]
    pub first: String,
    /// URL or request name
    #[arg()]
    pub second: String,
    /// Trailing params (headers/body data)
    #[arg(value_parser, trailing_var_arg = true)]
    pub params: Vec<String>,
    /// Send body as application/x-www-form-urlencoded instead of JSON
    #[arg(long, default_value_t = false)]
    pub form: bool,
    /// Print the full response (status, headers, body)
    #[arg(short, long, global = true)]
    pub verbose: bool,
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

pub fn handle_get(url: &str, params: &[String], verbose: bool, spinner_msg: &str) {
    let url = ensure_url_scheme(url);
    let (headers, _) = parse_params(params);
    let client = Client::new(ReqwestBackend);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = run_with_spinner(spinner_msg, || rt.block_on(client.get(&url, headers)));
    print_response(result, verbose);
}

pub fn handle_post(url: &str, params: &[String], form: bool, verbose: bool, spinner_msg: &str) {
    let url = ensure_url_scheme(url);
    let (mut headers, data) = parse_params(params);

    let body = if form {
        Client::<ReqwestBackend>::prepare_form_body(&data, &mut headers)
    } else {
        Client::<ReqwestBackend>::prepare_json_body(data, &mut headers)
    };
    let client = Client::new(ReqwestBackend);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = run_with_spinner(spinner_msg, || {
        rt.block_on(client.post(&url, &body, headers))
    });
    print_response(result, verbose);
}

pub fn handle_put(url: &str, params: &[String], form: bool, verbose: bool, spinner_msg: &str) {
    let url = ensure_url_scheme(url);
    let (mut headers, data) = parse_params(params);

    let body = if form {
        Client::<ReqwestBackend>::prepare_form_body(&data, &mut headers)
    } else {
        Client::<ReqwestBackend>::prepare_json_body(data, &mut headers)
    };
    let client = Client::new(ReqwestBackend);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = run_with_spinner(spinner_msg, || {
        rt.block_on(client.put(&url, &body, headers))
    });
    print_response(result, verbose);
}

pub fn handle_patch(url: &str, params: &[String], form: bool, verbose: bool, spinner_msg: &str) {
    let url = ensure_url_scheme(url);
    let (mut headers, data) = parse_params(params);

    let body = if form {
        Client::<ReqwestBackend>::prepare_form_body(&data, &mut headers)
    } else {
        Client::<ReqwestBackend>::prepare_json_body(data, &mut headers)
    };
    let client = Client::new(ReqwestBackend);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = run_with_spinner(spinner_msg, || {
        rt.block_on(client.patch(&url, &body, headers))
    });
    print_response(result, verbose);
}

pub fn handle_delete(url: &str, params: &[String], verbose: bool, spinner_msg: &str) {
    let url = ensure_url_scheme(url);
    let (headers, _) = parse_params(params);
    let client = Client::new(ReqwestBackend);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = run_with_spinner(spinner_msg, || rt.block_on(client.delete(&url, headers)));
    print_response(result, verbose);
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

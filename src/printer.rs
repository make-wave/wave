use crate::http_client::{HttpResponse, HttpError};
use anstyle::{AnsiColor, Style};
use std::io::{self, Write};

fn pretty_print_json_colored(value: &serde_json::Value) -> String {
    use colored_json::{Color, ColoredFormatter, PrettyFormatter, Styler};
    let styler = Styler {
        key: Color::Yellow.bold(),
        ..Default::default()
    };
    let formatter = ColoredFormatter::with_styler(PrettyFormatter::new(), styler);
    formatter
        .to_colored_json_auto(value)
        .unwrap_or_else(|_| serde_json::to_string_pretty(value).unwrap_or_default())
}

fn get_status_style(status: u16) -> Style {
    match status {
        200..=299 => Style::new()
            .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)))
            .bold(),
        300..=399 => Style::new()
            .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Yellow)))
            .bold(),
        400..=599 => Style::new()
            .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Red)))
            .bold(),
        _ => Style::new()
            .fg_color(Some(anstyle::Color::Ansi(AnsiColor::White)))
            .bold(),
    }
}

fn format_status_line(status: u16) -> String {
    let status_style = get_status_style(status);
    format!(
        "{}Status: {}{}\n",
        status_style.render(),
        status,
        anstyle::Reset.render()
    )
}

fn format_header(name: &str, value: &str) -> String {
    let key_style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Blue)));
    let value_style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::White)));
    format!(
        "{}{}: {}{}{}\n",
        key_style.render(),
        name,
        value_style.render(),
        value,
        anstyle::Reset.render()
    )
}

fn should_show_all_headers(verbose: bool, status: u16) -> bool {
    verbose || (status >= 400 && status <= 599)
}

fn format_all_headers(headers: &http::HeaderMap) -> String {
    let mut output = String::new();
    for (name, value) in headers {
        output.push_str(&format_header(
            name.as_str(),
            value.to_str().unwrap_or("<invalid header value>")
        ));
    }
    output
}

fn format_headers_section(resp: &HttpResponse, verbose: bool) -> (String, bool) {
    let mut output = String::new();
    let showed_headers = should_show_all_headers(verbose, resp.status);

    if showed_headers {
        output.push_str(&format_all_headers(&resp.headers));
    }

    (output, showed_headers)
}

fn format_content_type_if_needed(resp: &HttpResponse, is_json: bool, showed_headers: bool) -> String {
    if !is_json && !showed_headers {
        if let Some(value) = resp.headers.get("content-type") {
            return format_header(
                "Content-Type",
                value.to_str().unwrap_or("<invalid header value>")
            );
        }
    }
    String::new()
}

fn format_body(body: &str, parsed_json: Option<&serde_json::Value>) -> String {
    match parsed_json {
        Some(json) => pretty_print_json_colored(json),
        None => {
            let value_style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::White)));
            format!(
                "{}{}{}\n",
                value_style.render(),
                body,
                anstyle::Reset.render()
            )
        }
    }
}

pub fn format_response(resp: &HttpResponse, verbose: bool) -> String {
    let mut output = String::new();
    
    // Format status line
    output.push_str(&format_status_line(resp.status));
    
    // Parse JSON once and reuse the result
    let parsed_json = serde_json::from_str::<serde_json::Value>(&resp.body).ok();
    let is_json = parsed_json.is_some();
    
    // Format headers section
    let (headers_output, showed_headers) = format_headers_section(resp, verbose);
    output.push_str(&headers_output);
    
    // Show Content-Type if needed
    output.push_str(&format_content_type_if_needed(resp, is_json, showed_headers));
    
    // Format body using pre-parsed JSON
    output.push_str(&format_body(&resp.body, parsed_json.as_ref()));
    
    output
}

pub fn print_response(result: Result<HttpResponse, HttpError>, verbose: bool) {
    let _ = print_response_to(&mut io::stdout(), result, verbose);
}

fn print_response_to<W: Write>(
    writer: &mut W,
    result: Result<HttpResponse, HttpError>,
    verbose: bool,
) -> io::Result<()> {
    match result {
        Ok(resp) => {
            writeln!(writer, "{}", format_response(&resp, verbose))
        }
        Err(e) => {
            let style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Red)));
            writeln!(
                writer,
                "{}Error: {}{}",
                style.render(),
                e,
                anstyle::Reset.render()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderMap;
    
    #[test]
    fn test_format_status_color_2xx() {
        let resp = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: "{}".to_string(),
        };
        let output = format_response(&resp, false);
        assert!(output.contains("Status: 200"));
        assert!(output.contains(
            &anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Green)))
                .render()
                .to_string()
        ));
    }

    #[test]
    fn test_format_status_color_4xx() {
        let resp = HttpResponse {
            status: 404,
            headers: HeaderMap::new(),
            body: "{}".to_string(),
        };
        let output = format_response(&resp, false);
        assert!(output.contains("Status: 404"));
        assert!(output.contains(
            &anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(AnsiColor::Red)))
                .render()
                .to_string()
        ));
    }

    #[test]
    fn test_format_pretty_print_json() {
        let body = r#"{\"foo\":1,\"bar\":{\"baz\":2}}"#;
        let resp = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: body.to_string(),
        };
        let output = format_response(&resp, false);
        assert!(output.contains("foo"));
        assert!(output.contains("bar"));
        assert!(output.contains("baz"));
        assert!(output.contains("1"));
        assert!(output.contains("2"));
        assert!(output.contains("{")); // pretty JSON
    }

    #[test]
    fn test_format_headers_verbose() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        
        let resp = HttpResponse {
            status: 200,
            headers,
            body: "{}".to_string(),
        };
        let output = format_response(&resp, true);
        assert!(output.contains("content-type: "));
        assert!(output.contains("application/json"));
    }

    #[test]
    fn test_format_content_type_if_not_json() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "text/html".parse().unwrap());
        
        let resp = HttpResponse {
            status: 200,
            headers,
            body: "<html></html>".to_string(),
        };
        let output = format_response(&resp, false);
        assert!(output.contains("Content-Type: "));
        assert!(output.contains("text/html"));
        assert!(output.contains("<html></html>"));
    }

    #[test]
    fn test_format_headers_on_error_status() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("x-error", "Not Found".parse().unwrap());
        
        let resp = HttpResponse {
            status: 404,
            headers,
            body: "{}".to_string(),
        };
        let output = format_response(&resp, false);
        assert!(output.contains("content-type: "));
        assert!(output.contains("application/json"));
        assert!(output.contains("x-error: "));
        assert!(output.contains("Not Found"));
    }

    #[test]
    fn test_print_response_to_writer_trailing_newline() {
        let resp = HttpResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: "hello".to_string(),
        };
        let mut buf = Vec::new();
        print_response_to(&mut buf, Ok(resp), false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.ends_with('\n'));
    }
}

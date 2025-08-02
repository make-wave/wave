use crate::http_client::{HttpResponse, HttpError};
use anstyle::{AnsiColor, Style};

pub fn format_response(resp: &HttpResponse, verbose: bool) -> String {
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

    let mut output = String::new();
    let status_style = match resp.status {
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
    };
    let key_style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::Blue)));
    let value_style = Style::new().fg_color(Some(anstyle::Color::Ansi(AnsiColor::White)));
    output.push_str(&format!(
        "{}Status: {}{}\n",
        status_style.render(),
        resp.status,
        anstyle::Reset.render()
    ));
    let mut showed_headers = false;
    if verbose {
        for (name, value) in &resp.headers {
            output.push_str(&format!(
                "{}{}: {}{}{}\n",
                key_style.render(),
                name,
                value_style.render(),
                value,
                anstyle::Reset.render()
            ));
        }
        showed_headers = true;
    }
    let body = &resp.body;
    let is_json = serde_json::from_str::<serde_json::Value>(body).is_ok();
    // Show all headers if error status (4xx/5xx) and not already shown
    if !verbose && (resp.status >= 400 && resp.status <= 599) {
        for (name, value) in &resp.headers {
            output.push_str(&format!(
                "{}{}: {}{}{}\n",
                key_style.render(),
                name,
                value_style.render(),
                value,
                anstyle::Reset.render()
            ));
        }
        showed_headers = true;
    }
    // Show Content-Type if not JSON and not already shown
    if !is_json && !showed_headers {
        if let Some((name, value)) = resp
            .headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("content-type"))
        {
            output.push_str(&format!(
                "{}{}: {}{}{}\n",
                key_style.render(),
                name,
                value_style.render(),
                value,
                anstyle::Reset.render()
            ));
        }
    }
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(json) => {
            output.push_str(&pretty_print_json_colored(&json));
        }
        Err(_) => {
            output.push_str(&format!(
                "{}{}{}\n",
                value_style.render(),
                body,
                anstyle::Reset.render()
            ));
        }
    }
    output
}

pub fn print_response(result: Result<HttpResponse, HttpError>, verbose: bool) {
    let _ = print_response_to(&mut std::io::stdout(), result, verbose);
}

fn print_response_to<W: std::io::Write>(
    writer: &mut W,
    result: Result<HttpResponse, HttpError>,
    verbose: bool,
) -> std::io::Result<()> {
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
    #[test]
    fn test_format_status_color_2xx() {
        let resp = HttpResponse {
            status: 200,
            headers: vec![],
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
            headers: vec![],
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
            headers: vec![],
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
        let resp = HttpResponse {
            status: 200,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: "{}".to_string(),
        };
        let output = format_response(&resp, true);
        assert!(output.contains("Content-Type: "));
        assert!(output.contains("application/json"));
    }

    #[test]
    fn test_format_content_type_if_not_json() {
        let resp = HttpResponse {
            status: 200,
            headers: vec![("Content-Type".to_string(), "text/html".to_string())],
            body: "<html></html>".to_string(),
        };
        let output = format_response(&resp, false);
        assert!(output.contains("Content-Type: "));
        assert!(output.contains("text/html"));
        assert!(output.contains("<html></html>"));
    }

    #[test]
    fn test_format_headers_on_error_status() {
        let resp = HttpResponse {
            status: 404,
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("X-Error".to_string(), "Not Found".to_string()),
            ],
            body: "{}".to_string(),
        };
        let output = format_response(&resp, false);
        assert!(output.contains("Content-Type: "));
        assert!(output.contains("application/json"));
        assert!(output.contains("X-Error: "));
        assert!(output.contains("Not Found"));
    }

    #[test]
    fn test_print_response_to_writer_trailing_newline() {
        let resp = HttpResponse {
            status: 200,
            headers: vec![],
            body: "hello".to_string(),
        };
        let mut buf = Vec::new();
        print_response_to(&mut buf, Ok(resp), false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.ends_with('\n'));
    }
}

use clap::Parser;
use wave::collection::{load_collection, resolve_request_vars};
use wave::{handle_delete, handle_get, handle_patch, handle_post, handle_put, Cli};

fn main() {
    let cli = Cli::parse();

    // Validation: --form must appear before any headers/body params
    if let Some(form_pos) = cli.params.iter().position(|p| p == "--form") {
        if form_pos > 0 {
            // Build corrected command suggestion
            let mut corrected = vec![
                "wave".to_string(),
                cli.first.clone(),
                cli.second.clone(),
                "--form".to_string()
            ];
            corrected.extend(cli.params.iter().filter(|p| *p != "--form").cloned());
            use anstyle::{AnsiColor, Color, Style};

            let style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red))).bold();
            let reset = Style::new().render();
            let error_prefix = format!("{}Error:{} ", style.render(), reset);
            let reset = "\x1b[0m"; // ANSI reset

eprint!("{}{}", error_prefix, reset);
eprintln!(
    " --form must appear before any headers or body parameters.\n\nYour command:\n  wave {} {} {}\n\nCorrect usage:\n  {}\n",
    cli.first,
    cli.second,
    cli.params.join(" "),
    corrected.join(" ")
);
            std::process::exit(1);
        }
    }

    // Disambiguate: if first positional arg is an HTTP method, treat as HTTP command
    let http_methods = ["get", "post", "put", "patch", "delete"];
    if http_methods.contains(&cli.first.to_lowercase().as_str()) {
        // HTTP command: method, url, params
        let method = &cli.first;
        let url = &cli.second;
        let spinner_msg = format!(
            "{} {}{}{}",
            method,
            url,
            if cli.params.is_empty() { "" } else { " " },
            cli.params.join(" ")
        );
        match method.to_lowercase().as_str() {
            "get" => handle_get(url, &cli.params, cli.verbose, &spinner_msg),
            "post" => handle_post(url, &cli.params, cli.form, cli.verbose, &spinner_msg),
            "put" => handle_put(url, &cli.params, cli.form, cli.verbose, &spinner_msg),
            "patch" => handle_patch(url, &cli.params, cli.form, cli.verbose, &spinner_msg),
            "delete" => handle_delete(url, &cli.params, cli.verbose, &spinner_msg),
            _ => eprintln!("Unknown method: {method}"),
        }
        return;
    }

    // Otherwise, treat as collection
    let collection = &cli.first;
    let request = &cli.second;
    let yaml_path = format!(".wave/{collection}.yaml");
    let yml_path = format!(".wave/{collection}.yml");
    let coll_result = load_collection(&yaml_path).or_else(|_| load_collection(&yml_path));
    match coll_result {
        Ok(coll) => {
            let file_vars = coll.variables.unwrap_or_default();
            match coll.requests.iter().find(|r| r.name == *request) {
                Some(req) => match resolve_request_vars(req, &file_vars) {
                    Ok(resolved) => {
                        let client =
                            wave::http_client::Client::new(wave::http_client::ReqwestBackend);
                        let method = resolved.method.to_uppercase();
                        let verbose = cli.verbose;
                        let spinner_msg = format!("{method} {}", resolved.url);
                        match method.as_str() {
                            "GET" => {
                                let headers: Vec<(String, String)> =
                                    resolved.headers.unwrap_or_default().into_iter().collect();
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                let result = wave::run_with_spinner(&spinner_msg, || {
                                    rt.block_on(client.get(&resolved.url, headers))
                                });
                                wave::printer::print_response(result, verbose);
                            }
                            "DELETE" => {
                                let headers: Vec<(String, String)> =
                                    resolved.headers.unwrap_or_default().into_iter().collect();
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                let result = wave::run_with_spinner(&spinner_msg, || {
                                    rt.block_on(client.delete(&resolved.url, headers))
                                });
                                wave::printer::print_response(result, verbose);
                            }
                            "POST" | "PUT" | "PATCH" => {
                                let headers: Vec<(String, String)> =
                                    resolved.headers.unwrap_or_default().into_iter().collect();
                                let (body, _is_form) = match &resolved.body {
                                    Some(wave::collection::Body::Json(map)) => {
                                        let json_str = serde_json::to_string(&map)
                                            .unwrap_or_else(|_| "{}".to_string());
                                        (json_str, false)
                                    }
                                    Some(wave::collection::Body::Form(map)) => {
                                        let form_str = wave::http_client::Client::<
                                            wave::http_client::ReqwestBackend,
                                        >::prepare_form_body(
                                            &map.iter()
                                                .map(|(k, v)| (k.clone(), v.clone()))
                                                .collect::<Vec<_>>(),
                                            &mut headers.clone(),
                                        );
                                        (form_str, true)
                                    }
                                    None => ("".to_string(), false),
                                };
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                let result = match method.as_str() {
                                    "POST" => wave::run_with_spinner(&spinner_msg, || {
                                        rt.block_on(client.post(&resolved.url, &body, headers))
                                    }),
                                    "PUT" => wave::run_with_spinner(&spinner_msg, || {
                                        rt.block_on(client.put(&resolved.url, &body, headers))
                                    }),
                                    "PATCH" => wave::run_with_spinner(&spinner_msg, || {
                                        rt.block_on(client.patch(&resolved.url, &body, headers))
                                    }),
                                    _ => unreachable!(),
                                };
                                wave::printer::print_response(result, verbose);
                            }
                            _ => eprintln!("Unsupported method: {method}"),
                        }
                    }
                    Err(e) => eprintln!("Variable resolution error: {e}"),
                },
                None => {
                    eprintln!("Request '{request}' not found in collection '{collection}'.")
                }
            }
        }
        Err(e) => eprintln!("Failed to load collection '{collection}': {e}"),
    }
}


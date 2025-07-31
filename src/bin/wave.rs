use clap::Parser;
use wave::collection::{load_collection, resolve_request_vars};
use wave::{handle_delete, handle_get, handle_patch, handle_post, handle_put, Cli};

fn main() {
    let cli = Cli::parse();

    // Disambiguate: if first positional arg is an HTTP method, treat as HTTP command
    let http_methods = ["get", "post", "put", "patch", "delete"];
    if let Some(first) = cli.collection.as_ref() {
        if http_methods.contains(&first.to_lowercase().as_str()) {
            // HTTP command: method, url, params
            let method = first;
            let url = cli.request.as_ref().expect("URL required after method");
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
    }

    // Otherwise, if both collection and request are present, run collection logic
    if let (Some(collection), Some(request)) = (cli.collection.as_ref(), cli.request.as_ref()) {
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
        return;
    }

    // If neither, print help
    eprintln!("Usage:\n  wave <collection> <request>\n  wave <method> <url> [params...]\nTry --help for more information.");
}

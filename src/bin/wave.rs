use clap::Parser;
use wave::collection::{load_collection, resolve_request_vars};
use wave::{handle_delete, handle_get, handle_patch, handle_post, handle_put, Cli};

fn spinner_msg(method: &str, url: &str, params: &[String]) -> String {
    format!(
        "{} {}{}",
        method,
        url,
        if params.is_empty() { "" } else { " " },
    )
}

fn prepare_headers_and_body(
    resolved: &wave::collection::Request,
) -> (Vec<(String, String)>, String, bool) {
    let mut headers: Vec<(String, String)> = resolved
        .headers
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();
    match &resolved.body {
        Some(wave::collection::Body::Json(map)) => {
            fn yaml_to_json(val: &serde_yaml::Value) -> serde_json::Value {
                match val {
                    serde_yaml::Value::Null => serde_json::Value::Null,
                    serde_yaml::Value::Bool(b) => serde_json::Value::Bool(*b),
                    serde_yaml::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            serde_json::Value::Number(i.into())
                        } else if let Some(f) = n.as_f64() {
                            serde_json::Number::from_f64(f)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        } else {
                            serde_json::Value::Null
                        }
                    }
                    serde_yaml::Value::String(s) => serde_json::Value::String(s.clone()),
                    serde_yaml::Value::Sequence(seq) => {
                        serde_json::Value::Array(seq.iter().map(yaml_to_json).collect())
                    }
                    serde_yaml::Value::Mapping(map) => {
                        let mut obj = serde_json::Map::new();
                        for (k, v) in map {
                            let key = match k {
                                serde_yaml::Value::String(s) => s.clone(),
                                _ => serde_yaml::to_string(k).unwrap_or_default(),
                            };
                            obj.insert(key, yaml_to_json(v));
                        }
                        serde_json::Value::Object(obj)
                    }
                    _ => serde_json::Value::Null,
                }
            }
            let json_obj = serde_json::Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), yaml_to_json(v)))
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
        Some(wave::collection::Body::Form(map)) => {
            let mut header_map = http::HeaderMap::new();
            let form_str =
                wave::http_client::Client::<wave::http_client::ReqwestBackend>::prepare_form_body(
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

fn run_with_spinner_and_print<F>(spinner_msg: &str, verbose: bool, f: F)
where
    F: FnOnce() -> Result<wave::http_client::HttpResponse, wave::http_client::HttpError>,
{
    let result = wave::run_with_spinner(spinner_msg, f);
    wave::printer::print_response(result, verbose);
}

fn send_request(client: &wave::http_client::Client<wave::http_client::ReqwestBackend>, req: &wave::http_client::HttpRequest) -> Result<wave::http_client::HttpResponse, wave::http_client::HttpError> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(client.send(req))
}

fn main() {
    let cli = Cli::parse();
    use wave::Command;
    match cli.command {
        Command::Get {
            url,
            params,
            verbose,
        } => {
            let msg = spinner_msg("GET", &url, &params);
            handle_get(&url, &params, verbose, &msg);
        }
        Command::Post {
            url,
            params,
            form,
            verbose,
        } => {
            let msg = spinner_msg("POST", &url, &params);
            handle_post(&url, &params, form, verbose, &msg);
        }
        Command::Put {
            url,
            params,
            form,
            verbose,
        } => {
            let msg = spinner_msg("PUT", &url, &params);
            handle_put(&url, &params, form, verbose, &msg);
        }
        Command::Patch {
            url,
            params,
            form,
            verbose,
        } => {
            let msg = spinner_msg("PATCH", &url, &params);
            handle_patch(&url, &params, form, verbose, &msg);
        }
        Command::Delete {
            url,
            params,
            verbose,
        } => {
            let msg = spinner_msg("DELETE", &url, &params);
            handle_delete(&url, &params, verbose, &msg);
        }
        Command::Collection {
            collection,
            request,
            verbose,
        } => {
            let yaml_path = format!(".wave/{collection}.yaml");
            let yml_path = format!(".wave/{collection}.yml");
            let coll_result = load_collection(&yaml_path).or_else(|_| load_collection(&yml_path));
            match coll_result {
                Ok(coll) => {
                    let file_vars = coll.variables.unwrap_or_default();
                    match coll.requests.iter().find(|r| r.name == request) {
                        Some(req) => match resolve_request_vars(req, &file_vars) {
                            Ok(resolved) => {
                                let client = wave::http_client::Client::new(
                                    wave::http_client::ReqwestBackend,
                                );
                                let method = &resolved.method;
                                let spinner_msg = format!("{} {}", method, resolved.url);
                                match resolved.method {
                                    wave::http_client::HttpMethod::Get => {
                                        let headers: Vec<(String, String)> = resolved
                                            .headers
                                            .unwrap_or_default()
                                            .into_iter()
                                            .collect();
                                        let req = wave::http_client::HttpRequest::new_with_headers(&resolved.url, wave::http_client::HttpMethod::Get, None, headers);
                                        run_with_spinner_and_print(&spinner_msg, verbose, || send_request(&client, &req));
                                    }
                                    wave::http_client::HttpMethod::Delete => {
                                        let headers: Vec<(String, String)> = resolved
                                            .headers
                                            .unwrap_or_default()
                                            .into_iter()
                                            .collect();
                                        let req = wave::http_client::HttpRequest::new_with_headers(&resolved.url, wave::http_client::HttpMethod::Delete, None, headers);
                                        run_with_spinner_and_print(&spinner_msg, verbose, || send_request(&client, &req));
                                    }
                                    wave::http_client::HttpMethod::Post | wave::http_client::HttpMethod::Put | wave::http_client::HttpMethod::Patch => {
                                        let (headers, body, _is_form) = prepare_headers_and_body(&resolved);
                                        let req = wave::http_client::HttpRequest::new_with_headers(&resolved.url, resolved.method.clone(), Some(body), headers);
                                        run_with_spinner_and_print(&spinner_msg, verbose, || send_request(&client, &req));
                                    }
                                    _ => eprintln!("Unsupported method: {method}"),
                                }
                            }
                            Err(e) => eprintln!("Variable resolution error: {e}"),
                        },
                        None => {
                            eprintln!(
                                "Request '{request}' not found in collection '{collection}'."
                            );
                        }
                    }
                }
                Err(e) => eprintln!("Failed to load collection '{collection}': {e}"),
            }
        }
    }
}

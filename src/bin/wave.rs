use clap::Parser;
use wave::collection::{load_collection, resolve_request_vars};
use wave::{handle_delete, handle_get, handle_patch, handle_post, handle_put, Cli};

fn main() {
    let cli = Cli::parse();
    use wave::Command;
    match cli.command {
        Command::Get {
            url,
            params,
            verbose,
        } => {
            let spinner_msg = format!(
                "GET {}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" ")
            );
            handle_get(&url, &params, verbose, &spinner_msg);
        }
        Command::Post {
            url,
            params,
            form,
            verbose,
        } => {
            let spinner_msg = format!(
                "POST {}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" ")
            );
            handle_post(&url, &params, form, verbose, &spinner_msg);
        }
        Command::Put {
            url,
            params,
            form,
            verbose,
        } => {
            let spinner_msg = format!(
                "PUT {}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" ")
            );
            handle_put(&url, &params, form, verbose, &spinner_msg);
        }
        Command::Patch {
            url,
            params,
            form,
            verbose,
        } => {
            let spinner_msg = format!(
                "PATCH {}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" ")
            );
            handle_patch(&url, &params, form, verbose, &spinner_msg);
        }
        Command::Delete {
            url,
            params,
            verbose,
        } => {
            let spinner_msg = format!(
                "DELETE {}{}{}",
                url,
                if params.is_empty() { "" } else { " " },
                params.join(" ")
            );
            handle_delete(&url, &params, verbose, &spinner_msg);
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
                                let method = resolved.method.to_uppercase();
                                let spinner_msg = format!("{} {}", method, resolved.url);
                                match method.as_str() {
                                    "GET" => {
                                        let headers: Vec<(String, String)> = resolved
                                            .headers
                                            .unwrap_or_default()
                                            .into_iter()
                                            .collect();
                                        let rt = tokio::runtime::Runtime::new().unwrap();
                                        let result = wave::run_with_spinner(&spinner_msg, || {
                                            rt.block_on(client.get(&resolved.url, headers))
                                        });
                                        wave::printer::print_response(result, verbose);
                                    }
                                    "DELETE" => {
                                        let headers: Vec<(String, String)> = resolved
                                            .headers
                                            .unwrap_or_default()
                                            .into_iter()
                                            .collect();
                                        let rt = tokio::runtime::Runtime::new().unwrap();
                                        let result = wave::run_with_spinner(&spinner_msg, || {
                                            rt.block_on(client.delete(&resolved.url, headers))
                                        });
                                        wave::printer::print_response(result, verbose);
                                    }
                                    "POST" | "PUT" | "PATCH" => {
                                        let mut headers: Vec<(String, String)> = resolved
                                            .headers
                                            .unwrap_or_default()
                                            .into_iter()
                                            .collect();
                                        let (body, _is_form) = match &resolved.body {
                                            Some(wave::collection::Body::Json(map)) => {
                                                fn yaml_to_json(
                                                    val: &serde_yaml::Value,
                                                ) -> serde_json::Value
                                                {
                                                    match val {
                                                        serde_yaml::Value::Null => {
                                                            serde_json::Value::Null
                                                        }
                                                        serde_yaml::Value::Bool(b) => {
                                                            serde_json::Value::Bool(*b)
                                                        }
                                                        serde_yaml::Value::Number(n) => {
                                                            if let Some(i) = n.as_i64() {
                                                                serde_json::Value::Number(i.into())
                                                            } else if let Some(f) = n.as_f64() {
                                                                serde_json::Number::from_f64(f)
                                                                    .map(serde_json::Value::Number)
                                                                    .unwrap_or(
                                                                        serde_json::Value::Null,
                                                                    )
                                                            } else {
                                                                serde_json::Value::Null
                                                            }
                                                        }
                                                        serde_yaml::Value::String(s) => {
                                                            serde_json::Value::String(s.clone())
                                                        }
                                                        serde_yaml::Value::Sequence(seq) => {
                                                            serde_json::Value::Array(
                                                                seq.iter()
                                                                    .map(yaml_to_json)
                                                                    .collect(),
                                                            )
                                                        }
                                                        serde_yaml::Value::Mapping(map) => {
                                                            let mut obj = serde_json::Map::new();
                                                            for (k, v) in map {
                                                                let key = match k {
                                                                    serde_yaml::Value::String(
                                                                        s,
                                                                    ) => s.clone(),
                                                                    _ => serde_yaml::to_string(k)
                                                                        .unwrap_or_default(),
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
                                                if !headers.iter().any(|(k, _)| {
                                                    k.eq_ignore_ascii_case("content-type")
                                                }) {
                                                    headers.push((
                                                        "Content-Type".to_string(),
                                                        "application/json".to_string(),
                                                    ));
                                                }
                                                let json_str = serde_json::to_string(&json_obj)
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
                                                if !headers.iter().any(|(k, _)| {
                                                    k.eq_ignore_ascii_case("content-type")
                                                }) {
                                                    headers.push((
                                                        "Content-Type".to_string(),
                                                        "application/x-www-form-urlencoded"
                                                            .to_string(),
                                                    ));
                                                }
                                                (form_str, true)
                                            }
                                            None => ("".to_string(), false),
                                        };
                                        let rt = tokio::runtime::Runtime::new().unwrap();
                                        let result = match method.as_str() {
                                            "POST" => wave::run_with_spinner(&spinner_msg, || {
                                                rt.block_on(client.post(
                                                    &resolved.url,
                                                    &body,
                                                    headers,
                                                ))
                                            }),
                                            "PUT" => wave::run_with_spinner(&spinner_msg, || {
                                                rt.block_on(client.put(
                                                    &resolved.url,
                                                    &body,
                                                    headers,
                                                ))
                                            }),
                                            "PATCH" => wave::run_with_spinner(&spinner_msg, || {
                                                rt.block_on(client.patch(
                                                    &resolved.url,
                                                    &body,
                                                    headers,
                                                ))
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

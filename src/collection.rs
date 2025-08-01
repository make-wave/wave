use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Collection {
    pub variables: Option<HashMap<String, String>>,
    pub requests: Vec<Request>,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<Body>, // Body is now validated for mutual exclusivity
}

use serde::de::{self, Deserializer, MapAccess, Visitor};
use std::fmt;

#[derive(Debug)]
pub enum Body {
    Json(HashMap<String, serde_yaml::Value>),
    Form(HashMap<String, String>),
}

impl<'de> Deserialize<'de> for Body {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BodyVisitor;
        impl<'de> Visitor<'de> for BodyVisitor {
            type Value = Body;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map with either 'json' or 'form' key")
            }
            fn visit_map<A>(self, mut map: A) -> Result<Body, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut json: Option<HashMap<String, serde_yaml::Value>> = None;
                let mut form: Option<HashMap<String, String>> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "json" => {
                            if json.is_some() {
                                return Err(de::Error::duplicate_field("json"));
                            }
                            json = Some(map.next_value()?);
                        }
                        "form" => {
                            if form.is_some() {
                                return Err(de::Error::duplicate_field("form"));
                            }
                            form = Some(map.next_value()?);
                        }
                        other => {
                            return Err(de::Error::unknown_field(other, &["json", "form"]));
                        }
                    }
                }
                match (json, form) {
                    (Some(_), Some(_)) => {
                        Err(de::Error::custom("Only one of 'json' or 'form' can be used in the body of a request. Please specify either 'json' or 'form', not both."))
                    }
                    (Some(j), None) => Ok(Body::Json(j)),
                    (None, Some(f)) => Ok(Body::Form(f)),
                    (None, None) => Err(de::Error::custom("Body must contain either 'json' or 'form' key.")),
                }
            }
        }
        deserializer.deserialize_map(BodyVisitor)
    }
}

/// Load collection and parse yaml collection
pub fn load_collection(path: &str) -> Result<Collection, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let coll: Collection = serde_yaml::from_str(&content)?;
    Ok(coll)
}

/// Resolves variables in a string using file-defined and environment variables.
pub fn resolve_vars(input: &str, file_vars: &HashMap<String, String>) -> Result<String, String> {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' && chars.peek() == Some(&'{') {
            chars.next(); // skip '{'
            let mut var_name = String::new();
            while let Some(&next_c) = chars.peek() {
                if next_c == '}' {
                    chars.next();
                    break;
                }
                var_name.push(next_c);
                chars.next();
            }
            if let Some(env_var) = var_name.strip_prefix("env:") {
                match std::env::var(env_var) {
                    Ok(val) => result.push_str(&val),
                    Err(_) => return Err(format!("Missing environment variable: {env_var}")),
                }
            } else {
                match file_vars.get(&var_name) {
                    Some(val) => result.push_str(val),
                    None => return Err(format!("Missing variable: {var_name}")),
                }
            }
        } else {
            result.push(c);
        }
    }
    Ok(result)
}

/// Recursively resolves variables in all request fields
pub fn resolve_request_vars(
    req: &Request,
    file_vars: &HashMap<String, String>,
) -> Result<Request, String> {
    let url = resolve_vars(&req.url, file_vars)?;
    let headers = match &req.headers {
        Some(hs) => {
            let mut resolved = HashMap::new();
            for (k, v) in hs {
                resolved.insert(k.clone(), resolve_vars(v, file_vars)?);
            }
            Some(resolved)
        }
        None => None,
    };
    let body = match &req.body {
        Some(Body::Json(map)) => {
            let mut resolved = HashMap::new();
            for (k, v) in map {
                let resolved_value = match v {
                    serde_yaml::Value::String(s) => {
                        serde_yaml::Value::String(resolve_vars(s, file_vars)?)
                    }
                    other => other.clone(),
                };
                resolved.insert(k.clone(), resolved_value);
            }
            Some(Body::Json(resolved))
        }
        Some(Body::Form(map)) => {
            let mut resolved = HashMap::new();
            for (k, v) in map {
                resolved.insert(k.clone(), resolve_vars(v, file_vars)?);
            }
            Some(Body::Form(resolved))
        }
        None => None,
    };
    Ok(Request {
        name: req.name.clone(),
        method: req.method.clone(),
        url,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_load_collection_and_resolve_vars() {
        let yaml = r#"
variables:
  base_url: https://api.example.com
  user_id: 42
requests:
  - name: Get User
    method: GET
    url: ${base_url}/users/${user_id}
    headers:
      Authorization: Bearer ${env:TEST_TOKEN}
      Accept: application/json
  - name: Create User
    method: POST
    url: ${base_url}/users
    headers:
      Authorization: Bearer ${env:TEST_TOKEN}
      Content-Type: application/json
    body:
      json:
        name: Alice
        email: alice@example.com
"#;
        env::set_var("TEST_TOKEN", "secret123");
        let mut path = std::env::temp_dir();
        path.push("test_wave_collection.yaml");
        fs::write(&path, yaml).unwrap();
        env::set_var("TEST_TOKEN", "secret123");
        let coll = load_collection(path.to_str().unwrap()).unwrap();
        let file_vars = coll.variables.clone().unwrap();
        let req = coll.requests.iter().find(|r| r.name == "Get User").unwrap();
        let resolved = resolve_request_vars(req, &file_vars).unwrap();
        assert_eq!(resolved.url, "https://api.example.com/users/42");
        assert_eq!(
            resolved
                .headers
                .as_ref()
                .unwrap()
                .get("Authorization")
                .unwrap(),
            "Bearer secret123"
        );
        assert_eq!(
            resolved.headers.as_ref().unwrap().get("Accept").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_missing_env_var_error() {
        let file_vars = HashMap::new();
        let s = "${env:DOES_NOT_EXIST}";
        let err = resolve_vars(s, &file_vars).unwrap_err();
        assert!(err.contains("Missing environment variable"));
    }

    #[test]
    fn test_missing_file_var_error() {
        let file_vars = HashMap::new();
        let s = "${not_defined}";
        let err = resolve_vars(s, &file_vars).unwrap_err();
        assert!(err.contains("Missing variable"));
    }
}

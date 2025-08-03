use crate::http_client::HttpMethod;
use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::fs;

/// Converts a serde_yaml::Value to serde_json::Value for YAML-to-JSON conversion
pub fn yaml_to_json(val: &serde_yaml::Value) -> serde_json::Value {
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

#[derive(Debug, Deserialize)]
pub struct Collection {
    pub variables: Option<HashMap<String, String>>,
    pub requests: Vec<Request>,
}

#[derive(Debug)]
pub struct Request {
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<Body>, // Body is now validated for mutual exclusivity
}

impl<'de> Deserialize<'de> for Request {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RequestHelper {
            name: String,
            method: String,
            url: String,
            headers: Option<HashMap<String, String>>,
            body: Option<Body>,
        }

        let helper = RequestHelper::deserialize(deserializer)?;
        let method = helper
            .method
            .parse::<HttpMethod>()
            .map_err(|e| de::Error::custom(format!("Invalid HTTP method: {e}")))?;

        Ok(Request {
            name: helper.name,
            method,
            url: helper.url,
            headers: helper.headers,
            body: helper.body,
        })
    }
}

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
    let content = fs::read_to_string(path)?;
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
        fs::write(&path, yaml).expect("Test: Write test file");
        env::set_var("TEST_TOKEN", "secret123");
        let coll = load_collection(path.to_str().expect("Test: Valid path"))
            .expect("Test: Load collection");
        let file_vars = coll.variables.clone().expect("Test: Variables exist");
        let req = coll
            .requests
            .iter()
            .find(|r| r.name == "Get User")
            .expect("Test: Find request");
        let resolved = resolve_request_vars(req, &file_vars).expect("Test: Resolve variables");
        assert_eq!(resolved.url, "https://api.example.com/users/42");
        assert_eq!(
            resolved
                .headers
                .as_ref()
                .expect("Test: Headers exist")
                .get("Authorization")
                .expect("Test: Auth header exists"),
            "Bearer secret123"
        );
        assert_eq!(
            resolved
                .headers
                .as_ref()
                .expect("Test: Headers exist")
                .get("Accept")
                .expect("Test: Accept header exists"),
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

    #[test]
    fn test_yaml_to_json_conversion() {
        // Test null
        assert_eq!(
            yaml_to_json(&serde_yaml::Value::Null),
            serde_json::Value::Null
        );

        // Test boolean
        assert_eq!(
            yaml_to_json(&serde_yaml::Value::Bool(true)),
            serde_json::Value::Bool(true)
        );

        // Test string
        assert_eq!(
            yaml_to_json(&serde_yaml::Value::String("test".to_string())),
            serde_json::Value::String("test".to_string())
        );

        // Test number (integer)
        let yaml_num = serde_yaml::Value::Number(serde_yaml::Number::from(42));
        let json_result = yaml_to_json(&yaml_num);
        assert_eq!(json_result, serde_json::Value::Number(42.into()));

        // Test array
        let yaml_array = serde_yaml::Value::Sequence(vec![
            serde_yaml::Value::String("item1".to_string()),
            serde_yaml::Value::String("item2".to_string()),
        ]);
        let json_result = yaml_to_json(&yaml_array);
        assert_eq!(
            json_result,
            serde_json::Value::Array(vec![
                serde_json::Value::String("item1".to_string()),
                serde_json::Value::String("item2".to_string()),
            ])
        );

        // Test object
        let mut yaml_map = serde_yaml::Mapping::new();
        yaml_map.insert(
            serde_yaml::Value::String("key".to_string()),
            serde_yaml::Value::String("value".to_string()),
        );
        let yaml_obj = serde_yaml::Value::Mapping(yaml_map);
        let json_result = yaml_to_json(&yaml_obj);

        let mut expected_map = serde_json::Map::new();
        expected_map.insert(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        );
        assert_eq!(json_result, serde_json::Value::Object(expected_map));
    }
}

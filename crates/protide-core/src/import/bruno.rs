//! Bruno (.bru) file format import

use http_parser::{HttpMethod, KeyValue, Protocol, Request, Scripts};
use super::ImportResult;
use serde_json;

pub fn parse_bruno(content: &str) -> Result<ImportResult, String> {
    let mut result = ImportResult::new();
    let blocks = parse_blocks(content);

    let mut name = String::new();
    let mut method = String::from("GET");
    let mut url = String::new();
    let mut headers: Vec<KeyValue> = Vec::new();
    let mut body = String::new();
    let mut query_params: Vec<KeyValue> = Vec::new();
    let mut protocol: Option<Protocol> = None;
    let mut scripts = Scripts::default();
    let mut graphql_vars: Option<String> = None;

    for (block_name, lines) in &blocks {
        match block_name.as_str() {
            "meta" => {
                for line in lines {
                    if let Some((k, v)) = split_kv(line) {
                        match k.as_str() {
                            "name" => name = v,
                            "type" => {
                                if v == "graphql" {
                                    protocol = Some(Protocol::GraphQL);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "get" | "post" | "put" | "delete" | "patch" | "head" | "options" => {
                method = block_name.to_uppercase();
                for line in lines {
                    if let Some((k, v)) = split_kv(line) {
                        if k == "url" {
                            url = v;
                        }
                    }
                }
            }
            "headers" => {
                for line in lines {
                    if let Some((k, v)) = split_kv(line) {
                        let enabled = !k.starts_with('~');
                        let key = k.trim_start_matches('~').to_string();
                        headers.push(KeyValue { key, value: v, enabled });
                    }
                }
            }
            "query" => {
                for line in lines {
                    if let Some((k, v)) = split_kv(line) {
                        let enabled = !k.starts_with('~');
                        let key = k.trim_start_matches('~').to_string();
                        query_params.push(KeyValue { key, value: v, enabled });
                    }
                }
            }
            "auth:bearer" => {
                for line in lines {
                    if let Some((k, v)) = split_kv(line) {
                        if k == "token" {
                            headers.push(KeyValue {
                                key: "Authorization".to_string(),
                                value: format!("Bearer {}", v),
                                enabled: true,
                            });
                        }
                    }
                }
            }
            "auth:basic" => {
                let mut user = String::new();
                let mut pass = String::new();
                for line in lines {
                    if let Some((k, v)) = split_kv(line) {
                        match k.as_str() {
                            "username" => user = v,
                            "password" => pass = v,
                            _ => {}
                        }
                    }
                }
                if !user.is_empty() {
                    use base64::{Engine, engine::general_purpose::STANDARD};
                    let encoded = STANDARD.encode(format!("{}:{}", user, pass));
                    headers.push(KeyValue {
                        key: "Authorization".to_string(),
                        value: format!("Basic {}", encoded),
                        enabled: true,
                    });
                }
            }
            "auth:apikey" => {
                let mut key_name = String::new();
                let mut key_value = String::new();
                let mut placement = String::from("header");
                for line in lines {
                    if let Some((k, v)) = split_kv(line) {
                        match k.as_str() {
                            "key" => key_name = v,
                            "value" => key_value = v,
                            "placement" => placement = v,
                            _ => {}
                        }
                    }
                }
                if !key_name.is_empty() {
                    if placement == "query" {
                        query_params.push(KeyValue { key: key_name, value: key_value, enabled: true });
                    } else {
                        headers.push(KeyValue { key: key_name, value: key_value, enabled: true });
                    }
                }
            }
            "body:json" | "body:text" | "body:xml" => {
                body = lines.join("\n").trim().to_string();
            }
            "body:graphql" => {
                body = lines.join("\n").trim().to_string();
                protocol = Some(Protocol::GraphQL);
            }
            "body:graphql:vars" => {
                graphql_vars = Some(lines.join("\n").trim().to_string());
            }
            "body:multipart-form" => {
                let pairs: Vec<String> = lines
                    .iter()
                    .filter_map(|l| split_kv(l))
                    .filter(|(k, _)| !k.starts_with('~'))
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                body = pairs.join("&");
                headers.push(KeyValue {
                    key: "Content-Type".to_string(),
                    value: "multipart/form-data".to_string(),
                    enabled: true,
                });
            }
            "body:form-urlencoded" => {
                let pairs: Vec<String> = lines
                    .iter()
                    .filter_map(|l| split_kv(l))
                    .filter(|(k, _)| !k.starts_with('~'))
                    .map(|(k, v)| {
                        format!("{}={}",
                            urlencoding::encode(&k),
                            urlencoding::encode(&v))
                    })
                    .collect();
                body = pairs.join("&");
            }
            "script:pre-request" => {
                scripts.pre_script = Some(lines.join("\n").trim().to_string());
            }
            "script:post-response" => {
                scripts.post_script = Some(lines.join("\n").trim().to_string());
            }
            "tests" => {
                scripts.tests = Some(lines.join("\n").trim().to_string());
            }
            _ => {}
        }
    }

    if url.is_empty() {
        return Err("No URL found in Bruno file".to_string());
    }

    // Wrap GraphQL query in canonical {"query":...,"variables":...} JSON
    if protocol == Some(Protocol::GraphQL) && !body.is_empty() {
        let vars_val = graphql_vars
            .as_deref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok())
            .unwrap_or(serde_json::Value::Null);
        let wrapped = if vars_val.is_null() {
            serde_json::json!({ "query": body })
        } else {
            serde_json::json!({ "query": body, "variables": vars_val })
        };
        body = serde_json::to_string_pretty(&wrapped).unwrap_or(body);
    }

    let final_url = if query_params.is_empty() {
        url
    } else {
        let qs: String = query_params
            .iter()
            .filter(|kv| kv.enabled)
            .map(|kv| format!("{}={}", urlencoding::encode(&kv.key), urlencoding::encode(&kv.value)))
            .collect::<Vec<_>>()
            .join("&");
        if url.contains('?') { format!("{}&{}", url, qs) } else { format!("{}?{}", url, qs) }
    };

    let http_method = HttpMethod::from_str(&method).unwrap_or(HttpMethod::Get);
    let mut request = Request::new(http_method, final_url);
    if !name.is_empty() { request.meta.name = Some(name); }
    request.meta.protocol = protocol;
    request.headers = headers;
    request.body = if body.is_empty() { None } else { Some(body) };
    request.scripts = scripts;

    result.requests.push(request);
    Ok(result)
}

fn count_unquoted_braces(line: &str) -> (i32, i32) {
    let mut opens = 0i32;
    let mut closes = 0i32;
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if quote.is_some() => { chars.next(); }
            '\'' | '"' | '`' => {
                if quote == Some(c) { quote = None; } else if quote.is_none() { quote = Some(c); }
            }
            '{' if quote.is_none() => opens += 1,
            '}' if quote.is_none() => closes += 1,
            _ => {}
        }
    }
    (opens, closes)
}

fn parse_blocks(content: &str) -> Vec<(String, Vec<String>)> {
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();
    let mut depth = 0i32;

    for line in content.lines() {
        let trimmed = line.trim();

        if current_name.is_none() {
            // Block opener: "blockname {" (Bruno identifiers never contain spaces other than before `{`)
            if let Some(block_name) = trimmed.strip_suffix('{').map(|s| s.trim().to_string())
                && !block_name.is_empty()
                && !block_name.contains(' ')  // guard against JS lines like "if (x) {"
            {
                current_name = Some(block_name);
                current_lines.clear();
                depth = 1;
            }
        } else {
            let (opens, closes) = count_unquoted_braces(line);
            depth += opens - closes;

            if depth <= 0 {
                // Block closed (the closing `}` may have been on this line)
                blocks.push((current_name.take().unwrap(), current_lines.clone()));
                current_lines.clear();
                depth = 0;
            } else {
                current_lines.push(line.to_string());
            }
        }
    }

    blocks
}

/// Split `key: value` on the first colon. Skips blank lines and `#` comments.
fn split_kv(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (k, v) = trimmed.split_once(':')?;
    Some((k.trim().to_string(), v.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
meta {
  name: Get Users
  type: http
  seq: 1
}

get {
  url: https://api.example.com/users
  body: none
  auth: none
}

headers {
  Content-Type: application/json
  X-API-Key: secret
  ~X-Disabled: ignored
}
"#;

    const SAMPLE_AUTH: &str = r#"
meta {
  name: Auth Request
  type: http
  seq: 1
}

post {
  url: https://api.example.com/data
  body: json
  auth: bearer
}

auth:bearer {
  token: mytoken123
}

body:json {
  {"key": "value"}
}
"#;

    const SAMPLE_SCRIPTS: &str = r#"
meta {
  name: Scripted Request
  type: http
  seq: 1
}

get {
  url: https://api.example.com/users
  body: none
  auth: none
}

script:pre-request {
  bru.setVar("ts", Date.now());
}

script:post-response {
  bru.setVar("userId", res.body.id);
}

tests {
  test("status 200", function() {
    expect(res.status).to.equal(200);
  });
}
"#;

    #[test]
    fn test_parse_bruno_basic() {
        let result = parse_bruno(SAMPLE).unwrap();
        assert_eq!(result.requests.len(), 1);
        let req = &result.requests[0];
        assert_eq!(req.url, "https://api.example.com/users");
        assert_eq!(req.meta.name.as_deref(), Some("Get Users"));
        // Disabled header (~X-Disabled) must be imported with enabled=false
        let enabled: Vec<_> = req.headers.iter().filter(|h| h.enabled).collect();
        let disabled: Vec<_> = req.headers.iter().filter(|h| !h.enabled).collect();
        assert_eq!(enabled.len(), 2);
        assert_eq!(disabled.len(), 1);
        assert_eq!(disabled[0].key, "X-Disabled");
    }

    #[test]
    fn test_parse_blocks() {
        let blocks = parse_blocks(SAMPLE);
        let names: Vec<&str> = blocks.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"meta"));
        assert!(names.contains(&"get"));
        assert!(names.contains(&"headers"));
    }

    #[test]
    fn test_bearer_auth() {
        let result = parse_bruno(SAMPLE_AUTH).unwrap();
        let req = &result.requests[0];
        let auth = req.headers.iter().find(|h| h.key == "Authorization").unwrap();
        assert_eq!(auth.value, "Bearer mytoken123");
        assert!(req.body.is_some());
    }

    #[test]
    fn test_scripts() {
        let result = parse_bruno(SAMPLE_SCRIPTS).unwrap();
        let req = &result.requests[0];
        assert!(req.scripts.pre_script.is_some());
        assert!(req.scripts.post_script.is_some());
        assert!(req.scripts.tests.is_some());
        assert!(req.scripts.pre_script.as_ref().unwrap().contains("bru.setVar"));
    }

    #[test]
    fn test_apikey_query_placement() {
        let bru = r#"
meta {
  name: Apikey Query
  type: http
  seq: 1
}

get {
  url: https://api.example.com/data
  body: none
  auth: apikey
}

auth:apikey {
  key: api_key
  value: secret123
  placement: query
}
"#;
        let result = parse_bruno(bru).unwrap();
        let req = &result.requests[0];
        assert!(req.url.contains("api_key=secret123"), "query param missing from URL");
        assert!(!req.headers.iter().any(|h| h.key == "api_key"), "apikey must not be in headers");
    }

    #[test]
    fn test_graphql_with_variables() {
        let bru = r#"
meta {
  name: GQL Query
  type: graphql
  seq: 1
}

post {
  url: https://api.example.com/graphql
  body: graphql
  auth: none
}

body:graphql {
  query GetUser($id: ID!) { user(id: $id) { name } }
}

body:graphql:vars {
  {"id": "42"}
}
"#;
        let result = parse_bruno(bru).unwrap();
        let req = &result.requests[0];
        assert_eq!(req.meta.protocol, Some(Protocol::GraphQL));
        let body = req.body.as_ref().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(body).expect("body must be JSON");
        assert!(parsed.get("query").is_some());
        assert_eq!(parsed["variables"]["id"].as_str(), Some("42"));
    }

    #[test]
    fn test_multipart_form() {
        let bru = r#"
meta {
  name: Upload
  type: http
  seq: 1
}

post {
  url: https://api.example.com/upload
  body: multipart-form
  auth: none
}

body:multipart-form {
  name: John
  file: /path/to/file.txt
}
"#;
        let result = parse_bruno(bru).unwrap();
        let req = &result.requests[0];
        assert!(req.headers.iter().any(|h| h.key == "Content-Type" && h.value == "multipart/form-data"));
        assert!(req.body.as_ref().map(|b| b.contains("name=John")).unwrap_or(false));
    }
}

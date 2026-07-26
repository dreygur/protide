//! Bruno (.bru) file format import

use super::ImportResult;
use http_parser::{HttpMethod, KeyValue, Protocol, Request, Scripts};
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
    let mut path_params: Vec<KeyValue> = Vec::new();
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
                            "type" if v == "graphql" => protocol = Some(Protocol::GraphQL),
                            _ => {}
                        }
                    }
                }
            }
            "get" | "post" | "put" | "delete" | "patch" | "head" | "options" => {
                method = block_name.to_uppercase();
                if let Some(v) = find_value(lines, "url") {
                    url = v;
                }
            }
            "headers" => collect_kv(lines, &mut headers),
            // `query` is the legacy block name; bru-lang v2 writes `params:query`.
            "query" | "params:query" => collect_kv(lines, &mut query_params),
            "params:path" => collect_kv(lines, &mut path_params),
            "auth:bearer" => {
                if let Some(v) = find_value(lines, "token") {
                    headers.push(auth_header(format!("Bearer {}", v)));
                }
            }
            "auth:basic" => {
                let user = find_value(lines, "username").unwrap_or_default();
                if !user.is_empty() {
                    use base64::{Engine, engine::general_purpose::STANDARD};
                    let pass = find_value(lines, "password").unwrap_or_default();
                    let encoded = STANDARD.encode(format!("{}:{}", user, pass));
                    headers.push(auth_header(format!("Basic {}", encoded)));
                }
            }
            "auth:apikey" => {
                let key_name = find_value(lines, "key").unwrap_or_default();
                if !key_name.is_empty() {
                    let kv = KeyValue {
                        key: key_name,
                        value: find_value(lines, "value").unwrap_or_default(),
                        enabled: true,
                    };
                    if find_value(lines, "placement").as_deref() == Some("query") {
                        query_params.push(kv);
                    } else {
                        headers.push(kv);
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
                        format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v))
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

    let (url, path_warnings) = apply_path_params(&url, &path_params);
    result.warnings.extend(path_warnings);

    // Bruno mirrors `params:query` into the URL's query string, so only append
    // keys the URL does not already carry - otherwise every param is doubled.
    let existing = url.split_once('?').map_or("", |(_, q)| q);
    let qs: String = query_params
        .iter()
        .filter(|kv| {
            kv.enabled
                && !existing
                    .split('&')
                    .any(|pair| pair.split('=').next() == Some(kv.key.as_str()))
        })
        .map(|kv| {
            format!(
                "{}={}",
                urlencoding::encode(&kv.key),
                urlencoding::encode(&kv.value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    let final_url = match (qs.is_empty(), url.contains('?')) {
        (true, _) => url,
        (false, true) => format!("{}&{}", url, qs),
        (false, false) => format!("{}?{}", url, qs),
    };

    let http_method = HttpMethod::from_str(&method).unwrap_or(HttpMethod::Get);
    let mut request = Request::new(http_method, final_url);
    if !name.is_empty() {
        request.meta.name = Some(name);
    }
    request.meta.protocol = protocol;
    request.headers = headers;
    request.body = if body.is_empty() { None } else { Some(body) };
    request.scripts = scripts;

    result.requests.push(request);
    Ok(result)
}

/// First value for `key` among a block's lines.
fn find_value(lines: &[String], key: &str) -> Option<String> {
    lines
        .iter()
        .filter_map(|l| split_kv(l))
        .find_map(|(k, v)| (k == key).then_some(v))
}

fn auth_header(value: String) -> KeyValue {
    KeyValue {
        key: "Authorization".to_string(),
        value,
        enabled: true,
    }
}

/// Collect `key: value` lines, honouring Bruno's `~` disabled-entry prefix.
fn collect_kv(lines: &[String], out: &mut Vec<KeyValue>) {
    for line in lines {
        if let Some((k, v)) = split_kv(line) {
            out.push(KeyValue {
                key: k.trim_start_matches('~').to_string(),
                value: v,
                enabled: !k.starts_with('~'),
            });
        }
    }
}

/// Substitute Bruno `:name` path placeholders with their `params:path` values,
/// mirroring Bruno's own whole-segment matching (so `https://`, `host:8080` and
/// userinfo are never touched). Anything that cannot be paired up is reported
/// as a warning rather than dropped in silence.
fn apply_path_params(url: &str, params: &[KeyValue]) -> (String, Vec<String>) {
    let (path, suffix) = url.find(['?', '#']).map_or((url, ""), |i| url.split_at(i));
    let mut warnings = Vec::new();
    let mut used: Vec<&str> = Vec::new();
    let substituted =
        path.split('/')
            .map(|segment| {
                let Some(name) = segment.strip_prefix(':').filter(|n| {
                    !n.is_empty() && n.chars().all(|c| c.is_alphanumeric() || c == '_')
                }) else {
                    return segment.to_string();
                };
                match params.iter().find(|p| p.key == name) {
                    Some(p) => {
                        used.push(name);
                        p.value.clone()
                    }
                    None => {
                        warnings.push(format!("Path parameter `:{}` has no value", name));
                        segment.to_string()
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("/");
    for p in params.iter().filter(|p| !used.contains(&p.key.as_str())) {
        warnings.push(format!(
            "Path parameter `{}` matches no `:{}` placeholder in the URL",
            p.key, p.key
        ));
    }
    (format!("{}{}", substituted, suffix), warnings)
}

fn count_unquoted_braces(line: &str) -> (i32, i32) {
    let mut opens = 0i32;
    let mut closes = 0i32;
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if quote.is_some() => {
                chars.next();
            }
            '\'' | '"' | '`' => {
                if quote == Some(c) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(c);
                }
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
                && !block_name.contains(' ')
            // guard against JS lines like "if (x) {"
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

    // A block left open at EOF (truncated file, or a missing closing brace)
    // must still be emitted - dropping it silently loses whatever it held.
    if let Some(name) = current_name.take() {
        blocks.push((name, current_lines));
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
        let auth = req
            .headers
            .iter()
            .find(|h| h.key == "Authorization")
            .unwrap();
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
        assert!(
            req.scripts
                .pre_script
                .as_ref()
                .unwrap()
                .contains("bru.setVar")
        );
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
        assert!(
            req.url.contains("api_key=secret123"),
            "query param missing from URL"
        );
        assert!(
            !req.headers.iter().any(|h| h.key == "api_key"),
            "apikey must not be in headers"
        );
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
    fn test_unterminated_block_is_not_dropped() {
        // A truncated .bru file (or one whose last block simply lacks its
        // closing brace) used to have that whole block discarded, so the
        // headers below were silently lost while the request still imported
        // successfully - data loss with no error and no warning.
        let bru = "meta {\n  name: Truncated\n}\n\nget {\n  url: https://api.example.com/x\n}\n\nheaders {\n  X-Kept: yes\n";
        let result = parse_bruno(bru).unwrap();
        let req = &result.requests[0];
        assert!(
            req.headers.iter().any(|h| h.key == "X-Kept"),
            "headers from the unterminated block must survive: {:?}",
            req.headers
        );
    }

    #[test]
    fn test_degenerate_input_never_panics() {
        for input in [
            "",
            "   ",
            "\n\n\n",
            "{",
            "}",
            "meta {",
            "meta {\n}",
            "get {\n}",
            "get {\n  url:\n}",
            "headers {\n  novalue\n}",
            "body:json {\n  {\"a\": \"}\"}\n}",
            "\u{0}\u{1b}[31m",
            "🙂 {\n  x: 1\n}",
            "{\"info\": {\"name\": \"a postman file\"}}",
            &"{".repeat(5000),
            &"a {\n".repeat(2000),
        ] {
            let _ = parse_bruno(input);
        }
    }

    #[test]
    fn test_missing_url_is_an_error_not_an_empty_request() {
        let err = parse_bruno("meta {\n  name: No URL\n}\n").unwrap_err();
        assert!(err.contains("No URL"), "unexpected error: {}", err);
    }

    #[test]
    fn test_unicode_name_url_and_headers() {
        let bru = "meta {\n  name: ユーザー一覧 🙂\n}\n\nget {\n  url: https://例え.テスト/ユーザー\n}\n\nheaders {\n  X-Ünïcödé: välüe-🎉\n}\n";
        let result = parse_bruno(bru).unwrap();
        let req = &result.requests[0];
        assert_eq!(req.meta.name.as_deref(), Some("ユーザー一覧 🙂"));
        assert_eq!(req.url, "https://例え.テスト/ユーザー");
        assert!(
            req.headers
                .iter()
                .any(|h| h.key == "X-Ünïcödé" && h.value == "välüe-🎉")
        );
    }

    #[test]
    fn test_query_params_appended_to_url_that_already_has_a_query() {
        let bru = "meta {\n  name: Q\n}\n\nget {\n  url: https://api.example.com/s?a=1\n}\n\nquery {\n  b: 2\n  ~c: 3\n}\n";
        let result = parse_bruno(bru).unwrap();
        let url = &result.requests[0].url;
        assert!(url.contains("a=1"), "existing query lost: {}", url);
        assert!(url.contains("b=2"), "new query param missing: {}", url);
        assert!(
            !url.contains("c=3"),
            "disabled query param must not be sent: {}",
            url
        );
        assert_eq!(
            url.matches('?').count(),
            1,
            "malformed query string: {}",
            url
        );
    }

    #[test]
    fn test_body_with_braces_in_strings_is_preserved() {
        let bru = "meta {\n  name: Braces\n}\n\npost {\n  url: https://api.example.com/x\n}\n\nbody:json {\n  {\"tpl\": \"{{name}}\", \"brace\": \"}\"}\n}\n";
        let result = parse_bruno(bru).unwrap();
        let body = result.requests[0].body.as_deref().unwrap();
        assert!(body.contains("{{name}}"), "body truncated: {}", body);
        assert!(
            body.contains("\"brace\": \"}\""),
            "body truncated: {}",
            body
        );
    }

    #[test]
    fn test_graphql_with_invalid_variables_json_keeps_the_query() {
        let bru = "meta {\n  name: G\n  type: graphql\n}\n\npost {\n  url: https://api.example.com/graphql\n}\n\nbody:graphql {\n  query { me { id } }\n}\n\nbody:graphql:vars {\n  not json at all\n}\n";
        let result = parse_bruno(bru).unwrap();
        let body = result.requests[0].body.as_deref().unwrap();
        let parsed: serde_json::Value =
            serde_json::from_str(body).expect("body must still be valid JSON");
        assert!(
            parsed["query"].as_str().unwrap_or("").contains("me"),
            "query lost when variables were unparseable: {}",
            body
        );
    }

    #[test]
    fn test_basic_auth_without_password() {
        let bru = "meta {\n  name: B\n}\n\nget {\n  url: https://api.example.com/x\n}\n\nauth:basic {\n  username: alice\n}\n";
        let result = parse_bruno(bru).unwrap();
        let auth = result.requests[0]
            .headers
            .iter()
            .find(|h| h.key == "Authorization")
            .expect("basic auth header must be emitted");
        use base64::{Engine, engine::general_purpose::STANDARD};
        assert_eq!(auth.value, format!("Basic {}", STANDARD.encode("alice:")));
    }

    /// REGRESSION: bru-lang v2 carries path variable values in a `params:path`
    /// block, with the URL keeping the `:id` placeholder. Both that block and
    /// v2's `params:query` were unrecognised block names, so every path and
    /// query parameter value was dropped on import with no error and no
    /// warning - the request still imported "successfully", just wrong.
    ///
    /// FIXED: `params:path` values are substituted into the URL path using
    /// Bruno's own whole-segment matching, `params:query` is treated as an
    /// alias of the legacy `query` block, and anything that cannot be paired
    /// up is reported through `ImportResult::warnings`.
    #[test]
    fn test_bruno_path_params_are_imported() {
        let bru = "meta {\n  name: P\n}\n\nget {\n  url: https://api.example.com/users/:id\n}\n\nparams:path {\n  id: 42\n}\n";
        let result = parse_bruno(bru).unwrap();
        assert_eq!(result.requests[0].url, "https://api.example.com/users/42");
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    }

    #[test]
    fn test_params_query_block_is_an_alias_of_query() {
        let bru = "meta {\n  name: Q2\n}\n\nget {\n  url: https://api.example.com/s\n}\n\nparams:query {\n  page: 1\n  ~limit: 20\n}\n";
        let url = &parse_bruno(bru).unwrap().requests[0].url;
        assert_eq!(url, "https://api.example.com/s?page=1");
    }

    /// Bruno writes `params:query` entries into the URL string as well, so a
    /// naive append doubles every parameter (`?limit=3&limit=3`).
    #[test]
    fn test_params_query_mirrored_in_url_is_not_duplicated() {
        let bru = "meta {\n  name: M\n}\n\nget {\n  url: https://api.example.com/todos/:num?limit=3\n}\n\nparams:query {\n  limit: 3\n  page: 2\n}\n\nparams:path {\n  num: 1\n}\n";
        let url = &parse_bruno(bru).unwrap().requests[0].url;
        assert_eq!(url, "https://api.example.com/todos/1?limit=3&page=2");
    }

    #[test]
    fn test_path_params_substitute_every_placeholder_and_leave_the_rest_alone() {
        let bru = "meta {\n  name: Multi\n}\n\nget {\n  url: https://user:pw@api.example.com:8443/o/:orgId/u/:userId/raw?a=:notAParam\n}\n\nparams:path {\n  orgId: acme\n  userId: 7\n}\n";
        let result = parse_bruno(bru).unwrap();
        assert_eq!(
            result.requests[0].url,
            "https://user:pw@api.example.com:8443/o/acme/u/7/raw?a=:notAParam"
        );
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    }

    #[test]
    fn test_unpaired_path_params_are_warned_about_not_dropped() {
        let bru = "meta {\n  name: W\n}\n\nget {\n  url: https://api.example.com/users/:id/posts/:postId\n}\n\nparams:path {\n  id: 42\n  stray: 9\n}\n";
        let result = parse_bruno(bru).unwrap();
        // The unfilled placeholder stays in the URL rather than being blanked.
        assert_eq!(
            result.requests[0].url,
            "https://api.example.com/users/42/posts/:postId"
        );
        assert!(
            result.warnings.iter().any(|w| w.contains(":postId")),
            "missing value not reported: {:?}",
            result.warnings
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("stray")),
            "unused path param not reported: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_path_params_with_unicode_and_no_url_query() {
        let bru = "meta {\n  name: U\n}\n\nget {\n  url: https://api.example.com/ユーザー/:名前\n}\n\nparams:path {\n  名前: 🙂\n}\n";
        let result = parse_bruno(bru).unwrap();
        assert_eq!(
            result.requests[0].url,
            "https://api.example.com/ユーザー/🙂"
        );
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
        assert!(
            req.headers
                .iter()
                .any(|h| h.key == "Content-Type" && h.value == "multipart/form-data")
        );
        assert!(
            req.body
                .as_ref()
                .map(|b| b.contains("name=John"))
                .unwrap_or(false)
        );
    }
}

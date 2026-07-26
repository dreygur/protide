use std::time::{Duration, Instant};

use super::{ExecutionBody, ExecutionMode, FormPartValue};

/// Raw HTTP response before scripting/extraction
#[derive(Debug)]
pub struct RawResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub time: Duration,
    pub size: usize,
}

/// `(url, headers, body)` as resolved for the wire by [`resolve_request`].
type ResolvedRequest = (String, Vec<(String, String)>, ExecutionBody);

/// Resolve URL, headers, and body for the given execution mode.
/// GraphQL wraps the query into a JSON body and injects Content-Type.
/// Errors if `variables` is non-empty but not valid JSON (empty defaults to `{}`).
fn resolve_request(
    url: &str,
    headers: &[(String, String)],
    body: &ExecutionBody,
    mode: &ExecutionMode,
) -> Result<ResolvedRequest, String> {
    match mode {
        ExecutionMode::GraphQL {
            query,
            variables,
            operation_name,
        } => {
            let vars: serde_json::Value = if variables.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(variables)
                    .map_err(|e| format!("Invalid GraphQL variables JSON: {}", e))?
            };
            let mut gql_body = serde_json::json!({
                "query": query,
                "variables": vars,
            });
            if let Some(op) = operation_name
                && !op.is_empty()
            {
                gql_body["operationName"] = serde_json::Value::String(op.clone());
            }
            let mut hdrs = headers.to_vec();
            if !hdrs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            {
                hdrs.push(("Content-Type".to_string(), "application/json".to_string()));
            }
            Ok((
                url.to_string(),
                hdrs,
                ExecutionBody::Text(gql_body.to_string()),
            ))
        }
        ExecutionMode::Http => Ok((url.to_string(), headers.to_vec(), body.clone())),
    }
}

/// Chrome 131 browser header fingerprint (Windows, en-US).
/// Applied when `impersonate_browser` is true.  Existing user-supplied values
/// for the same header names are preserved (user headers take precedence).
const CHROME_PROFILE: &[(&str, &str)] = &[
    (
        "User-Agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
    ),
    (
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7",
    ),
    ("Accept-Language", "en-US,en;q=0.9"),
    ("Accept-Encoding", "gzip, deflate, br, zstd"),
    (
        "sec-ch-ua",
        "\"Google Chrome\";v=\"131\", \"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\"",
    ),
    ("sec-ch-ua-mobile", "?0"),
    ("sec-ch-ua-platform", "\"Windows\""),
    ("Upgrade-Insecure-Requests", "1"),
    ("sec-fetch-dest", "document"),
    ("sec-fetch-mode", "navigate"),
    ("sec-fetch-site", "none"),
    ("sec-fetch-user", "?1"),
];

/// Build the header list for a request with the Chrome browser profile prepended.
/// User-supplied headers override matching profile entries so explicit values win.
fn apply_browser_profile(user_headers: &[(String, String)]) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = CHROME_PROFILE
        .iter()
        .filter(|(name, _)| {
            !user_headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(name))
        })
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    result.extend_from_slice(user_headers);
    result
}

/// Execute a blocking HTTP (or GraphQL-over-HTTP) request.
pub fn run_http(
    url: &str,
    method: &str,
    headers: &[(String, String)],
    body: &ExecutionBody,
    mode: &ExecutionMode,
    timeout_secs: u64,
    verify_ssl: bool,
    impersonate_browser: bool,
) -> Result<RawResponse, String> {
    let start = Instant::now();
    let (resolved_url, mut resolved_headers, resolved_body) =
        resolve_request(url, headers, body, mode)?;

    if impersonate_browser {
        resolved_headers = apply_browser_profile(&resolved_headers);
    }

    let req_method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| format!("Invalid HTTP method '{}': {}", method, e))?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(!verify_ssl)
        .build()
        .map_err(|e| e.to_string())?;
    let mut req_builder = client.request(req_method, &resolved_url);

    let is_multipart = matches!(resolved_body, ExecutionBody::Multipart(_));
    for (key, value) in &resolved_headers {
        if is_multipart && key.eq_ignore_ascii_case("content-type") {
            continue;
        }
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    match &resolved_body {
        ExecutionBody::None => {}
        ExecutionBody::Text(s) => {
            req_builder = req_builder.body(s.clone());
        }
        ExecutionBody::Binary(bytes) => {
            req_builder = req_builder.body(bytes.clone());
        }
        ExecutionBody::Multipart(parts) => {
            let mut form = reqwest::blocking::multipart::Form::new();
            for part in parts {
                match &part.value {
                    FormPartValue::Text(v) => {
                        form = form.text(part.name.clone(), v.clone());
                    }
                    FormPartValue::File(path) => {
                        // A file that can't be opened must abort the request:
                        // silently dropping the part would send a request that
                        // looks successful but is missing the attachment.
                        let file_part =
                            reqwest::blocking::multipart::Part::file(path).map_err(|e| {
                                format!("Failed to attach file '{}': {}", path.display(), e)
                            })?;
                        form = form.part(part.name.clone(), file_part);
                    }
                }
            }
            req_builder = req_builder.multipart(form);
        }
    }

    let response = req_builder.send().map_err(|e| e.to_string())?;
    let elapsed = start.elapsed();
    let status = response.status().as_u16();
    let status_text = status_text(status).to_string();
    let resp_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body_str = response.text().unwrap_or_default();
    let size = body_str.len();

    Ok(RawResponse {
        status,
        status_text,
        headers: resp_headers,
        body: body_str,
        time: elapsed,
        size,
    })
}

pub(crate) fn status_text(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::FormPart;
    use crate::execution::test_server::{TestServer, json_response, response};
    use crate::test_support::TempDir;

    /// Send a plain HTTP request with the defaults every test wants:
    /// a short timeout, TLS verification on, no browser impersonation.
    fn send(
        url: &str,
        method: &str,
        headers: &[(String, String)],
        body: &ExecutionBody,
    ) -> Result<RawResponse, String> {
        run_http(
            url,
            method,
            headers,
            body,
            &ExecutionMode::Http,
            5,
            true,
            false,
        )
    }

    fn hdr(k: &str, v: &str) -> (String, String) {
        (k.to_string(), v.to_string())
    }

    // ── Request line, headers, query ─────────────────────────────────────────

    #[test]
    fn sends_method_target_and_headers_verbatim() {
        let server = TestServer::json("{}");
        let headers = vec![hdr("X-Custom", "abc"), hdr("Accept", "application/json")];
        send(
            &server.url("/users/1"),
            "DELETE",
            &headers,
            &ExecutionBody::None,
        )
        .expect("send");

        let req = server.only_request();
        assert_eq!(req.method, "DELETE");
        assert_eq!(req.target, "/users/1");
        assert_eq!(req.header("X-Custom"), Some("abc"));
        assert_eq!(req.header("Accept"), Some("application/json"));
        assert_eq!(req.body, "");
    }

    /// Query strings are the transport for API-key-in-query auth, so they must
    /// reach the wire byte-for-byte, percent-encoding and all.
    #[test]
    fn preserves_query_string_and_percent_encoding() {
        let server = TestServer::json("{}");
        let target = "/search?api_key=k%2Fey%3D&q=hello%20world&empty=&flag";
        send(&server.url(target), "GET", &[], &ExecutionBody::None).expect("send");
        assert_eq!(server.only_request().target, target);
    }

    /// Duplicate header names (Cookie, Set-Cookie style) must all be sent,
    /// not collapsed to the last one.
    #[test]
    fn sends_repeated_headers_once_each() {
        let server = TestServer::json("{}");
        let headers = vec![hdr("X-Tag", "one"), hdr("X-Tag", "two")];
        send(&server.url("/"), "GET", &headers, &ExecutionBody::None).expect("send");
        assert_eq!(server.only_request().header_values("X-Tag"), ["one", "two"]);
    }

    /// This layer receives values that the caller has already substituted; an
    /// unresolved `{{var}}` must be passed through untouched rather than being
    /// mangled or dropped, so the failure is visible to the user.
    #[test]
    fn passes_unsubstituted_variable_placeholder_through() {
        let server = TestServer::json("{}");
        let headers = vec![hdr("Authorization", "Bearer {{token}}")];
        send(&server.url("/"), "GET", &headers, &ExecutionBody::None).expect("send");
        assert_eq!(
            server.only_request().header("Authorization"),
            Some("Bearer {{token}}")
        );
    }

    #[test]
    fn rejects_invalid_http_method() {
        let err = send(
            "http://127.0.0.1:1/",
            "BAD METHOD",
            &[],
            &ExecutionBody::None,
        )
        .expect_err("invalid method must error");
        assert!(err.contains("BAD METHOD"), "unhelpful error: {err}");
    }

    #[test]
    fn rejects_unparsable_url() {
        send("not-a-url", "GET", &[], &ExecutionBody::None).expect_err("bad URL must error");
    }

    // ── Bodies ───────────────────────────────────────────────────────────────

    #[test]
    fn sends_text_body_with_length_and_content_type() {
        let server = TestServer::json("{}");
        let headers = vec![hdr("Content-Type", "application/json")];
        let body = ExecutionBody::Text(r#"{"name":"Ada"}"#.to_string());
        send(&server.url("/users"), "POST", &headers, &body).expect("send");

        let req = server.only_request();
        assert_eq!(req.method, "POST");
        assert_eq!(req.body, r#"{"name":"Ada"}"#);
        assert_eq!(req.header("Content-Type"), Some("application/json"));
        assert_eq!(req.header("Content-Length"), Some("14"));
    }

    /// A UTF-8 body must be length-prefixed in bytes, not characters, or the
    /// server truncates it.
    #[test]
    fn sends_multibyte_body_with_byte_length() {
        let server = TestServer::json("{}");
        let body = ExecutionBody::Text("{\"t\":\"héllo — 日本\"}".to_string());
        send(&server.url("/"), "POST", &[], &body).expect("send");

        let req = server.only_request();
        assert_eq!(req.body, "{\"t\":\"héllo — 日本\"}");
        assert_eq!(
            req.header("Content-Length").map(str::to_string),
            Some(req.body.len().to_string())
        );
    }

    #[test]
    fn sends_binary_body_verbatim() {
        let server = TestServer::json("{}");
        let body = ExecutionBody::Binary(b"\x01\x02rawbytes".to_vec());
        send(&server.url("/"), "PUT", &[], &body).expect("send");
        assert_eq!(server.only_request().body, "\u{1}\u{2}rawbytes");
    }

    #[test]
    fn none_body_sends_no_payload() {
        let server = TestServer::json("{}");
        send(&server.url("/"), "POST", &[], &ExecutionBody::None).expect("send");
        assert_eq!(server.only_request().body, "");
    }

    /// Multipart bodies must carry the generated boundary; a user-supplied
    /// Content-Type has to be dropped or the boundary parameter is lost and
    /// the server can't parse the payload.
    #[test]
    fn multipart_overrides_user_content_type_and_sends_parts() {
        let server = TestServer::json("{}");
        let dir = TempDir::new("protide_http_multipart");
        let file = dir.write("note.txt", b"file-contents");

        let headers = vec![hdr("Content-Type", "application/json")];
        let body = ExecutionBody::Multipart(vec![
            FormPart {
                name: "field".to_string(),
                value: FormPartValue::Text("value".to_string()),
            },
            FormPart {
                name: "upload".to_string(),
                value: FormPartValue::File(file),
            },
        ]);
        send(&server.url("/upload"), "POST", &headers, &body).expect("send");

        let req = server.only_request();
        let content_type = req.header("Content-Type").expect("content-type");
        assert!(
            content_type.starts_with("multipart/form-data; boundary="),
            "unexpected content-type: {content_type}"
        );
        assert_eq!(req.header_values("Content-Type").len(), 1);
        assert!(req.body.contains(r#"name="field""#), "body: {}", req.body);
        assert!(req.body.contains("value"));
        assert!(req.body.contains(r#"filename="note.txt""#));
        assert!(req.body.contains("file-contents"));
    }

    /// A multipart part whose file can't be opened must fail the request.
    /// Silently dropping it would send a request that looks fine but is
    /// missing the attachment entirely.
    #[test]
    fn multipart_missing_file_fails_the_request() {
        let dir = TempDir::new("protide_http_missing");
        let missing = dir.path().join("nope.bin");
        let body = ExecutionBody::Multipart(vec![FormPart {
            name: "upload".to_string(),
            value: FormPartValue::File(missing),
        }]);
        let err = send("http://127.0.0.1:1/", "POST", &[], &body)
            .expect_err("missing attachment must error");
        assert!(err.contains("nope.bin"), "unhelpful error: {err}");
    }

    // ── GraphQL mode ─────────────────────────────────────────────────────────

    #[test]
    fn graphql_wraps_query_and_injects_json_content_type() {
        let server = TestServer::json(r#"{"data":{}}"#);
        let mode = ExecutionMode::GraphQL {
            query: "query Q { me { id } }".to_string(),
            variables: r#"{"id":1}"#.to_string(),
            operation_name: Some("Q".to_string()),
        };
        run_http(
            &server.url("/graphql"),
            "POST",
            &[],
            &ExecutionBody::None,
            &mode,
            5,
            true,
            false,
        )
        .expect("send");

        let req = server.only_request();
        assert_eq!(req.header("Content-Type"), Some("application/json"));
        let sent: serde_json::Value = serde_json::from_str(&req.body).expect("valid JSON body");
        assert_eq!(sent["query"], "query Q { me { id } }");
        assert_eq!(sent["variables"]["id"], 1);
        assert_eq!(sent["operationName"], "Q");
    }

    #[test]
    fn graphql_defaults_variables_to_empty_object() {
        let (_, headers, body) = resolve_request(
            "http://example.invalid/",
            &[],
            &ExecutionBody::None,
            &ExecutionMode::GraphQL {
                query: "{ me }".to_string(),
                variables: "   ".to_string(),
                operation_name: None,
            },
        )
        .expect("resolve");

        let sent: serde_json::Value =
            serde_json::from_str(&body.as_text().expect("text body")).expect("valid JSON");
        assert_eq!(sent["variables"], serde_json::json!({}));
        assert!(sent.get("operationName").is_none());
        assert_eq!(headers, vec![hdr("Content-Type", "application/json")]);
    }

    #[test]
    fn graphql_rejects_invalid_variables_json() {
        let err = resolve_request(
            "http://example.invalid/",
            &[],
            &ExecutionBody::None,
            &ExecutionMode::GraphQL {
                query: "{ me }".to_string(),
                variables: "{not json".to_string(),
                operation_name: None,
            },
        )
        .expect_err("invalid variables must error");
        assert!(err.contains("GraphQL variables"), "unhelpful error: {err}");
    }

    /// An explicit Content-Type (e.g. `application/graphql+json`) must win
    /// over the injected default, and must not be duplicated.
    #[test]
    fn graphql_keeps_user_content_type() {
        let user = vec![hdr("content-type", "application/graphql+json")];
        let (_, headers, _) = resolve_request(
            "http://example.invalid/",
            &user,
            &ExecutionBody::None,
            &ExecutionMode::GraphQL {
                query: "{ me }".to_string(),
                variables: String::new(),
                operation_name: None,
            },
        )
        .expect("resolve");
        assert_eq!(headers, user);
    }

    #[test]
    fn graphql_omits_empty_operation_name() {
        let (_, _, body) = resolve_request(
            "http://example.invalid/",
            &[],
            &ExecutionBody::None,
            &ExecutionMode::GraphQL {
                query: "{ me }".to_string(),
                variables: String::new(),
                operation_name: Some(String::new()),
            },
        )
        .expect("resolve");
        let sent: serde_json::Value =
            serde_json::from_str(&body.as_text().expect("text body")).expect("valid JSON");
        assert!(sent.get("operationName").is_none());
    }

    #[test]
    fn http_mode_passes_url_headers_and_body_through() {
        let headers = vec![hdr("A", "1")];
        let (url, resolved, body) = resolve_request(
            "http://example.invalid/x",
            &headers,
            &ExecutionBody::Text("raw".into()),
            &ExecutionMode::Http,
        )
        .expect("resolve");
        assert_eq!(url, "http://example.invalid/x");
        assert_eq!(resolved, headers);
        assert_eq!(body.as_text().as_deref(), Some("raw"));
    }

    // ── Browser impersonation ────────────────────────────────────────────────

    #[test]
    fn browser_profile_is_applied_but_user_headers_win() {
        let server = TestServer::json("{}");
        let headers = vec![hdr("user-agent", "protide-test/1.0")];
        run_http(
            &server.url("/"),
            "GET",
            &headers,
            &ExecutionBody::None,
            &ExecutionMode::Http,
            5,
            true,
            true,
        )
        .expect("send");

        let req = server.only_request();
        assert_eq!(req.header_values("User-Agent"), ["protide-test/1.0"]);
        assert_eq!(req.header("Accept-Language"), Some("en-US,en;q=0.9"));
        assert_eq!(req.header("sec-ch-ua-mobile"), Some("?0"));
    }

    #[test]
    fn browser_profile_is_absent_when_not_impersonating() {
        let server = TestServer::json("{}");
        send(&server.url("/"), "GET", &[], &ExecutionBody::None).expect("send");
        assert_eq!(server.only_request().header("sec-ch-ua-mobile"), None);
    }

    #[test]
    fn apply_browser_profile_dedupes_case_insensitively() {
        let user = vec![hdr("ACCEPT", "text/plain")];
        let merged = apply_browser_profile(&user);
        let accepts: Vec<&str> = merged
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("accept"))
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(accepts, ["text/plain"]);
        assert_eq!(merged.len(), CHROME_PROFILE.len());
    }

    // ── Response metadata ────────────────────────────────────────────────────

    #[test]
    fn reports_status_headers_body_and_size() {
        let body = r#"{"ok":true}"#;
        let server = TestServer::spawn(vec![response(
            201,
            &[("Content-Type", "application/json"), ("X-Trace", "t-1")],
            body,
        )]);
        let resp = send(&server.url("/"), "POST", &[], &ExecutionBody::None).expect("send");

        assert_eq!(resp.status, 201);
        assert_eq!(resp.status_text, "Created");
        assert_eq!(resp.body, body);
        assert_eq!(resp.size, body.len());
        assert!(resp.time < Duration::from_secs(5), "time: {:?}", resp.time);
        assert_eq!(
            resp.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("x-trace"))
                .map(|(_, v)| v.as_str()),
            Some("t-1")
        );
    }

    /// An HTTP error status is a successful exchange, not a transport error:
    /// the body must still be delivered so the user can read the error.
    #[test]
    fn error_status_is_returned_with_body_not_as_error() {
        let server = TestServer::spawn(vec![response(500, &[], "boom")]);
        let resp = send(&server.url("/"), "GET", &[], &ExecutionBody::None).expect("send");
        assert_eq!(resp.status, 500);
        assert_eq!(resp.status_text, "Internal Server Error");
        assert_eq!(resp.body, "boom");
    }

    #[test]
    fn connection_closed_without_response_is_an_error() {
        let server = TestServer::spawn(vec![String::new()]);
        send(&server.url("/"), "GET", &[], &ExecutionBody::None)
            .expect_err("closed connection must error");
    }

    /// A server that accepts but never answers must be cut off by the
    /// configured timeout rather than hanging the request thread.
    #[test]
    fn honours_request_timeout() {
        let server = TestServer::stalled();
        let start = Instant::now();
        let err = run_http(
            &server.url("/"),
            "GET",
            &[],
            &ExecutionBody::None,
            &ExecutionMode::Http,
            1,
            true,
            false,
        )
        .expect_err("must time out or fail");
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "took {:?}",
            start.elapsed()
        );
        assert!(!err.is_empty());
    }

    // ── Redirects ────────────────────────────────────────────────────────────

    #[test]
    fn follows_redirect_to_final_response() {
        let server = TestServer::spawn(vec![
            response(302, &[("Location", "/final")], ""),
            json_response(r#"{"done":true}"#),
        ]);
        let resp = send(&server.url("/start"), "GET", &[], &ExecutionBody::None).expect("send");

        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, r#"{"done":true}"#);
        let reqs = server.requests();
        assert_eq!(reqs.len(), 2);
        assert_eq!(reqs[0].target, "/start");
        assert_eq!(reqs[1].target, "/final");
    }

    /// Credentials must not follow a redirect to a different origin - that
    /// would hand the user's token to whatever host the server names.
    #[test]
    fn does_not_forward_authorization_across_origins() {
        let target = TestServer::json(r#"{"leak":"check"}"#);
        let origin = TestServer::spawn(vec![response(
            302,
            &[("Location", target.url("/final").as_str())],
            "",
        )]);

        let headers = vec![hdr("Authorization", "Bearer super-secret")];
        send(&origin.url("/start"), "GET", &headers, &ExecutionBody::None).expect("send");

        let forwarded = target.only_request();
        assert_eq!(
            forwarded.header("Authorization"),
            None,
            "Authorization leaked to a different origin across a redirect"
        );
    }

    // ── Status text table ────────────────────────────────────────────────────

    #[test]
    fn status_text_maps_known_codes() {
        assert_eq!(status_text(200), "OK");
        assert_eq!(status_text(404), "Not Found");
        assert_eq!(status_text(503), "Service Unavailable");
    }

    #[test]
    fn status_text_falls_back_for_unknown_codes() {
        assert_eq!(status_text(599), "Unknown");
    }
}

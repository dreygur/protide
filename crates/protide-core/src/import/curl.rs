//! cURL command parser
//!
//! Parses cURL commands into HTTP requests.

use http_parser::{HttpMethod, KeyValue, Request, RequestMeta};

use super::ImportResult;

/// Parse a cURL command into requests
pub fn parse_curl(input: &str) -> Result<ImportResult, String> {
    let mut result = ImportResult::new();

    // Join backslash-continued physical lines into logical lines first, so
    // that when splitting multiple curl commands apart (below) a
    // continuation line (which doesn't start with "curl ") stays attached to
    // the command it belongs to instead of being silently dropped.
    let mut logical_lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for line in input.lines() {
        let trimmed_end = line.trim_end();
        if let Some(stripped) = trimmed_end.strip_suffix('\\') {
            current.push_str(stripped);
            current.push(' ');
        } else {
            current.push_str(trimmed_end);
            logical_lines.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        logical_lines.push(current);
    }

    // Handle multiple curl commands separated by newlines
    for line in &logical_lines {
        let trimmed = line.trim();
        if trimmed.starts_with("curl ") || trimmed.starts_with("curl\t") {
            match parse_single_curl(trimmed) {
                Ok(request) => result.add_request(request),
                Err(e) => result.add_warning(e),
            }
        }
    }

    // If no requests parsed from lines, try parsing entire input as one command
    if result.requests.is_empty() {
        let request = parse_single_curl(input.trim())?;
        result.add_request(request);
    }

    Ok(result)
}

/// Parse a single cURL command
fn parse_single_curl(input: &str) -> Result<Request, String> {
    let args = parse_curl_args(input)?;

    let mut method = HttpMethod::Get;
    let mut url = String::new();
    let mut headers: Vec<KeyValue> = Vec::new();
    let mut body: Option<String> = None;
    let mut name: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];

        match arg.as_str() {
            "-X" | "--request" => {
                i += 1;
                if i < args.len() {
                    method = HttpMethod::from_str(&args[i])
                        .ok_or_else(|| format!("Unknown HTTP method: {}", args[i]))?;
                }
            }
            "-H" | "--header" => {
                i += 1;
                if i < args.len()
                    && let Some((key, value)) = parse_header(&args[i])
                {
                    headers.push(KeyValue::new(key, value));
                }
            }
            // Repeated data flags are concatenated with '&', as real curl does.
            "-d" | "--data" | "--data-raw" | "--data-binary" | "-F" | "--form"
            | "--form-string" => {
                i += 1;
                if i < args.len() {
                    append_body(&mut body, &args[i]);
                    // Implicitly use POST if no method specified
                    if method == HttpMethod::Get {
                        method = HttpMethod::Post;
                    }
                }
            }
            "--data-urlencode" => {
                i += 1;
                if i < args.len() {
                    append_body(&mut body, &urlencoding::encode(&args[i]));
                    if method == HttpMethod::Get {
                        method = HttpMethod::Post;
                    }
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < args.len() {
                    // Basic auth: user:password
                    let auth = base64_encode(&args[i]);
                    headers.push(KeyValue::new("Authorization", format!("Basic {}", auth)));
                }
            }
            "-A" | "--user-agent" => {
                i += 1;
                if i < args.len() {
                    headers.push(KeyValue::new("User-Agent", args[i].clone()));
                }
            }
            "-e" | "--referer" => {
                i += 1;
                if i < args.len() {
                    headers.push(KeyValue::new("Referer", args[i].clone()));
                }
            }
            "-b" | "--cookie" => {
                i += 1;
                if i < args.len() {
                    headers.push(KeyValue::new("Cookie", args[i].clone()));
                }
            }
            "--compressed" => {
                // Add Accept-Encoding header
                if !headers
                    .iter()
                    .any(|h| h.key.eq_ignore_ascii_case("Accept-Encoding"))
                {
                    headers.push(KeyValue::new("Accept-Encoding", "gzip, deflate, br"));
                }
            }
            "-I" | "--head" => {
                method = HttpMethod::Head;
            }
            "-G" | "--get" => {
                method = HttpMethod::Get;
            }
            // Ignored flags
            "-k" | "--insecure" | "-s" | "--silent" | "-S" | "--show-error" | "-L"
            | "--location" | "-v" | "--verbose" | "-i" | "--include" | "-o" | "--output" | "-O"
            | "--remote-name" | "--connect-timeout" | "-m" | "--max-time" | "--retry" => {
                // Some of these take arguments
                if matches!(
                    arg.as_str(),
                    "-o" | "--output" | "--connect-timeout" | "-m" | "--max-time" | "--retry"
                ) {
                    i += 1; // Skip the argument
                }
            }
            _ => {
                // Check if it's a URL (doesn't start with -).
                //
                // Values of flags we don't recognise arrive here as bare
                // arguments, and the "contains a dot" heuristic happily
                // accepts things like "photo.jpg" or "proxy.internal:8080".
                // Keep the first URL found, and only let a later argument
                // replace it if that one carries an explicit scheme and the
                // current one doesn't - otherwise a stray flag value
                // silently redirects the imported request.
                if !arg.starts_with('-') && (arg.contains("://") || arg.contains('.')) {
                    let upgrades_scheme = arg.contains("://") && !url.contains("://");
                    if url.is_empty() || upgrades_scheme {
                        url = arg.clone();
                        // Try to extract name from URL
                        name = extract_name_from_url(&url);
                    }
                }
            }
        }
        i += 1;
    }

    if url.is_empty() {
        return Err("No URL found in cURL command".to_string());
    }

    // Add http:// if no protocol specified
    if !url.contains("://") {
        url = format!("http://{}", url);
    }

    let mut request = Request::new(method, url);
    request.headers = headers;
    request.body = body;
    request.meta = RequestMeta {
        name,
        ..Default::default()
    };

    Ok(request)
}

/// Append a data segment to the body, joining repeated segments with '&'.
fn append_body(body: &mut Option<String>, part: &str) {
    match body {
        Some(existing) => {
            existing.push('&');
            existing.push_str(part);
        }
        None => *body = Some(part.to_string()),
    }
}

/// Parse cURL command into arguments, handling quotes
fn parse_curl_args(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    // Skip "curl" prefix
    let input = input.trim_start_matches("curl").trim_start();

    for ch in input.chars() {
        if escape_next {
            escape_next = false;
            if ch != '\n' {
                current.push(ch);
            }
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' | '\n' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    if in_single_quote {
        return Err("Unterminated single quote".to_string());
    }
    if in_double_quote {
        return Err("Unterminated double quote".to_string());
    }

    Ok(args)
}

/// Parse a header string "Key: Value" into (key, value)
fn parse_header(header: &str) -> Option<(String, String)> {
    let colon_pos = header.find(':')?;
    let key = header[..colon_pos].trim().to_string();
    let value = header[colon_pos + 1..].trim().to_string();
    Some((key, value))
}

/// Base64 encode a string
fn base64_encode(input: &str) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD};
    STANDARD.encode(input.as_bytes())
}

/// Extract a name from URL path
fn extract_name_from_url(url: &str) -> Option<String> {
    let url = url::Url::parse(url).ok()?;
    let path = url.path();

    // Get last non-empty segment
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if let Some(last) = segments.last() {
        // Remove file extension if present
        let name = last.split('.').next().unwrap_or(last);
        if !name.is_empty() && name != "api" && name != "v1" && name != "v2" {
            return Some(name.to_string());
        }
    }

    // Fall back to host
    url.host_str().map(|h| h.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_get() {
        let result = parse_curl("curl https://api.example.com/users").unwrap();
        assert_eq!(result.requests.len(), 1);
        let req = &result.requests[0];
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.url, "https://api.example.com/users");
    }

    #[test]
    fn test_post_with_data() {
        let result = parse_curl(r#"curl -X POST -H "Content-Type: application/json" -d '{"name":"test"}' https://api.example.com/users"#).unwrap();
        assert_eq!(result.requests.len(), 1);
        let req = &result.requests[0];
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.body, Some(r#"{"name":"test"}"#.to_string()));
        assert!(
            req.headers
                .iter()
                .any(|h| h.key == "Content-Type" && h.value == "application/json")
        );
    }

    #[test]
    fn test_headers() {
        let result = parse_curl(r#"curl -H "Authorization: Bearer token123" -H "Accept: application/json" https://api.example.com"#).unwrap();
        let req = &result.requests[0];
        assert!(
            req.headers
                .iter()
                .any(|h| h.key == "Authorization" && h.value == "Bearer token123")
        );
        assert!(
            req.headers
                .iter()
                .any(|h| h.key == "Accept" && h.value == "application/json")
        );
    }

    #[test]
    fn test_basic_auth() {
        let result = parse_curl(r#"curl -u "user:password" https://api.example.com"#).unwrap();
        let req = &result.requests[0];
        assert!(
            req.headers
                .iter()
                .any(|h| h.key == "Authorization" && h.value.starts_with("Basic "))
        );
    }

    #[test]
    fn test_implicit_post() {
        let result = parse_curl(r#"curl -d "data=value" https://api.example.com"#).unwrap();
        let req = &result.requests[0];
        assert_eq!(req.method, HttpMethod::Post);
    }

    #[test]
    fn test_quoted_args() {
        let args = parse_curl_args(
            r#"-H "Content-Type: application/json" -d '{"key": "value"}' https://example.com"#,
        )
        .unwrap();
        assert!(args.contains(&"Content-Type: application/json".to_string()));
        assert!(args.contains(&r#"{"key": "value"}"#.to_string()));
    }

    #[test]
    fn test_multiple_multiline_curl_commands_lose_headers() {
        // Two curl commands, each using backslash line-continuation (as browsers'
        // "copy as cURL" commonly produce). parse_curl() first joins
        // backslash-continued physical lines into logical lines, then splits
        // on lines starting with "curl ", so each command's continuation line
        // stays attached to the right command and no headers are lost.
        let cmd = "curl https://api.example.com/a \\\n  -H \"X-Test: 1\"\ncurl https://api.example.com/b \\\n  -H \"X-Test: 2\"";
        let result = parse_curl(cmd).unwrap();
        assert_eq!(
            result.requests.len(),
            2,
            "expected 2 requests, got {:?}",
            result.requests
        );
        assert!(
            result.requests[0]
                .headers
                .iter()
                .any(|h| h.key == "X-Test" && h.value == "1"),
            "expected X-Test header on first request, got {:?}",
            result.requests[0].headers
        );
        assert!(
            result.requests[1]
                .headers
                .iter()
                .any(|h| h.key == "X-Test" && h.value == "2"),
            "expected X-Test header on second request, got {:?}",
            result.requests[1].headers
        );
    }

    #[test]
    fn test_form_upload_does_not_overwrite_the_url() {
        // `-F` was not recognised, so its value fell through to the URL
        // heuristic (it contains a '.') and replaced the real URL that had
        // already been parsed - the request silently pointed at
        // "http://file=@photo.jpg" and the form data was dropped entirely.
        let result =
            parse_curl(r#"curl https://api.example.com/upload -F "file=@photo.jpg""#).unwrap();
        let req = &result.requests[0];
        assert_eq!(
            req.url, "https://api.example.com/upload",
            "form field must not be mistaken for the URL"
        );
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(
            req.body.as_deref(),
            Some("file=@photo.jpg"),
            "form data must not be dropped"
        );
    }

    #[test]
    fn test_url_is_not_overwritten_by_unknown_flag_values() {
        // An unrecognised flag that takes a value (here `--proxy`) leaves
        // its value as a bareword argument; it must not displace the URL.
        let result =
            parse_curl("curl https://api.example.com/users --proxy proxy.internal:8080").unwrap();
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
    }

    #[test]
    fn test_scheme_bearing_url_wins_over_earlier_bareword() {
        let result = parse_curl("curl -F upload.txt https://api.example.com/users").unwrap();
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
    }

    #[test]
    fn test_repeated_data_flags_are_concatenated() {
        // Real curl joins repeated -d values with '&'. Keeping only the last
        // one silently drops the rest of the payload.
        let result = parse_curl("curl -d a=1 -d b=2 -d c=3 https://api.example.com").unwrap();
        assert_eq!(result.requests[0].body.as_deref(), Some("a=1&b=2&c=3"));
    }

    #[test]
    fn test_empty_and_degenerate_input_never_panics() {
        for input in [
            "",
            "   ",
            "\n\n",
            "curl",
            "curl ",
            "curl -X",
            "curl -H",
            "curl -d",
            "curl -u",
            "curl --data-urlencode",
            "curl -X POST",
            "not a curl command at all",
            "curl '",
            "curl \"",
            "curl \\",
            "curl -H 'X: 1' \u{0}",
            "🙂",
        ] {
            // Must return Ok or Err, never panic.
            let _ = parse_curl(input);
        }
    }

    #[test]
    fn test_unterminated_quote_is_an_error_not_a_panic() {
        assert!(parse_curl("curl 'https://api.example.com").is_err());
        assert!(parse_curl("curl \"https://api.example.com").is_err());
    }

    #[test]
    fn test_missing_url_is_an_error() {
        let err = parse_curl("curl -X POST -H 'Accept: */*'").unwrap_err();
        assert!(err.contains("No URL"), "unexpected error: {}", err);
    }

    #[test]
    fn test_unicode_url_and_headers_survive() {
        let result =
            parse_curl("curl 'https://例え.テスト/ユーザー?q=🙂' -H 'X-Ünïcödé: välüe-🎉'")
                .unwrap();
        let req = &result.requests[0];
        assert_eq!(req.url, "https://例え.テスト/ユーザー?q=🙂");
        assert!(
            req.headers
                .iter()
                .any(|h| h.key == "X-Ünïcödé" && h.value == "välüe-🎉")
        );
    }

    #[test]
    fn test_header_value_may_contain_colons() {
        let result =
            parse_curl("curl https://api.example.com -H 'X-Time: 12:30:00 +00:00'").unwrap();
        assert_eq!(result.requests[0].headers[0].value, "12:30:00 +00:00");
    }

    #[test]
    fn test_one_bad_command_does_not_drop_the_others() {
        let input = "curl https://api.example.com/a\ncurl -X NOPE https://api.example.com/b\ncurl https://api.example.com/c";
        let result = parse_curl(input).unwrap();
        assert_eq!(result.requests.len(), 2, "good commands must still import");
        assert_eq!(
            result.warnings.len(),
            1,
            "the failed command must be reported, not silently dropped"
        );
    }

    #[test]
    fn test_multiline_backslash_continuation() {
        let cmd = "curl -X POST \\\n  -H \"Content-Type: application/json\" \\\n  https://api.example.com/users";
        let result = parse_curl(cmd).unwrap();
        assert_eq!(result.requests.len(), 1);
        let req = &result.requests[0];
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.url, "https://api.example.com/users");
        assert!(req.headers.iter().any(|h| h.key == "Content-Type"));
    }
}

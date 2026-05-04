//! cURL command parser
//!
//! Parses cURL commands into HTTP requests.

use http_parser::{HttpMethod, KeyValue, Request, RequestMeta};

use super::ImportResult;

/// Parse a cURL command into requests
pub fn parse_curl(input: &str) -> Result<ImportResult, String> {
    let mut result = ImportResult::new();

    // Handle multiple curl commands separated by newlines
    for line in input.lines() {
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
                if i < args.len() {
                    if let Some((key, value)) = parse_header(&args[i]) {
                        headers.push(KeyValue::new(key, value));
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if i < args.len() {
                    body = Some(args[i].clone());
                    // Implicitly use POST if no method specified
                    if method == HttpMethod::Get {
                        method = HttpMethod::Post;
                    }
                }
            }
            "--data-urlencode" => {
                i += 1;
                if i < args.len() {
                    // URL encode the data
                    let encoded = urlencoding::encode(&args[i]).to_string();
                    body = Some(match body {
                        Some(existing) => format!("{}&{}", existing, encoded),
                        None => encoded,
                    });
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
                if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("Accept-Encoding")) {
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
            "-k" | "--insecure" | "-s" | "--silent" | "-S" | "--show-error" |
            "-L" | "--location" | "-v" | "--verbose" | "-i" | "--include" |
            "-o" | "--output" | "-O" | "--remote-name" | "--connect-timeout" |
            "-m" | "--max-time" | "--retry" => {
                // Some of these take arguments
                if matches!(arg.as_str(), "-o" | "--output" | "--connect-timeout" | "-m" | "--max-time" | "--retry") {
                    i += 1; // Skip the argument
                }
            }
            _ => {
                // Check if it's a URL (doesn't start with -)
                if !arg.starts_with('-') && (arg.starts_with("http://") || arg.starts_with("https://") || arg.contains("://") || arg.contains('.')) {
                    url = arg.clone();
                    // Try to extract name from URL
                    if name.is_none() {
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
            current.push(ch);
            escape_next = false;
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
        assert!(req.headers.iter().any(|h| h.key == "Content-Type" && h.value == "application/json"));
    }

    #[test]
    fn test_headers() {
        let result = parse_curl(r#"curl -H "Authorization: Bearer token123" -H "Accept: application/json" https://api.example.com"#).unwrap();
        let req = &result.requests[0];
        assert!(req.headers.iter().any(|h| h.key == "Authorization" && h.value == "Bearer token123"));
        assert!(req.headers.iter().any(|h| h.key == "Accept" && h.value == "application/json"));
    }

    #[test]
    fn test_basic_auth() {
        let result = parse_curl(r#"curl -u "user:password" https://api.example.com"#).unwrap();
        let req = &result.requests[0];
        assert!(req.headers.iter().any(|h| h.key == "Authorization" && h.value.starts_with("Basic ")));
    }

    #[test]
    fn test_implicit_post() {
        let result = parse_curl(r#"curl -d "data=value" https://api.example.com"#).unwrap();
        let req = &result.requests[0];
        assert_eq!(req.method, HttpMethod::Post);
    }

    #[test]
    fn test_quoted_args() {
        let args = parse_curl_args(r#"-H "Content-Type: application/json" -d '{"key": "value"}' https://example.com"#).unwrap();
        assert!(args.contains(&"Content-Type: application/json".to_string()));
        assert!(args.contains(&r#"{"key": "value"}"#.to_string()));
    }
}

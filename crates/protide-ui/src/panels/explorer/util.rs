/// Fallback used when a name sanitizes down to something unusable.
const FALLBACK_NAME: &str = "untitled";

/// Sanitize a string to be used as a single filename component.
///
/// The result is always a plain name that stays inside the directory it is
/// joined onto: separators become `-`, and the pure-navigation components
/// `.`/`..` (and the empty string) are replaced outright.
pub fn sanitize_filename(name: &str) -> String {
    let cleaned = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string();
    // `""`, `.` and `..` are not names - joining them onto a directory either
    // fails or escapes it, letting an imported collection write outside the
    // workspace.
    if matches!(cleaned.as_str(), "" | "." | "..") {
        return FALLBACK_NAME.to_string();
    }
    cleaned
}

/// Convert a Request to .http file content
pub fn request_to_http_content(request: &http_parser::Request) -> Result<String, String> {
    let mut content = String::new();

    if let Some(name) = &request.meta.name {
        content.push_str(&format!("# @name {}\n", name));
    }

    if let Some(desc) = &request.meta.description {
        content.push_str(&format!(
            "# @description {}\n",
            desc.lines().next().unwrap_or("")
        ));
    }

    if !content.is_empty() {
        content.push('\n');
    }

    content.push_str(&format!("{} {}\n", request.method.as_str(), request.url));

    for header in &request.headers {
        if header.enabled {
            content.push_str(&format!("{}: {}\n", header.key, header.value));
        }
    }

    if let Some(body) = &request.body {
        content.push('\n');
        content.push_str(body);
        if !body.ends_with('\n') {
            content.push('\n');
        }
    }

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_parser::{HttpMethod, KeyValue, Request};
    use std::path::{Component, Path, PathBuf};

    fn request(method: HttpMethod, url: &str) -> Request {
        Request::new(method, url.to_string())
    }

    fn header(key: &str, value: &str, enabled: bool) -> KeyValue {
        KeyValue {
            key: key.to_string(),
            value: value.to_string(),
            enabled,
        }
    }

    // ── sanitize_filename ────────────────────────────────────────────────────

    #[test]
    fn ordinary_names_are_left_alone() {
        for name in [
            "users",
            "Get User",
            "user-42",
            "日本語のリクエスト",
            "a.b.c",
        ] {
            assert_eq!(sanitize_filename(name), name);
        }
    }

    #[test]
    fn path_separators_and_reserved_characters_become_dashes() {
        assert_eq!(sanitize_filename("a/b"), "a-b");
        assert_eq!(sanitize_filename("a\\b"), "a-b");
        assert_eq!(sanitize_filename("C:\\Users"), "C--Users");
        assert_eq!(sanitize_filename("who?"), "who-");
        assert_eq!(sanitize_filename("a*b\"c<d>e|f"), "a-b-c-d-e-f");
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(sanitize_filename("  users  "), "users");
        assert_eq!(sanitize_filename("\t\nusers\n"), "users");
    }

    // FIXED: `sanitize_filename` replaced separators but let `.` and `..`
    // through untouched, and could return an empty string. Import writes
    // `output_dir.join(sanitize_filename(collection_name))` and
    // `collection_dir.join(sanitize_filename(folder_name))`
    // (explorer/workspace_io.rs:37,53), so an imported Postman/OpenAPI
    // collection containing a folder literally named ".." wrote its .http files
    // one directory *above* the workspace - a directory-traversal write driven
    // by untrusted file content. An empty name was almost as bad: it produced a
    // dotfile (".http") that `scan_directory` then hides, so the import looked
    // like it had silently done nothing.
    #[test]
    fn navigation_components_never_escape_the_target_directory() {
        for name in ["..", ".", "", "   ", "../", "..\\", "/..", "  ..  "] {
            let sanitized = sanitize_filename(name);
            assert!(!sanitized.is_empty(), "{name:?} sanitized to an empty name");
            let joined = Path::new("/workspace/collection").join(&sanitized);
            assert!(
                joined.starts_with("/workspace/collection"),
                "{name:?} -> {sanitized:?} escaped to {}",
                joined.display()
            );
            assert!(
                !joined
                    .components()
                    .any(|c| matches!(c, Component::ParentDir | Component::CurDir)),
                "{name:?} -> {sanitized:?} kept a navigation component"
            );
        }
    }

    #[test]
    fn a_sanitized_name_is_always_exactly_one_path_component() {
        for name in [
            "a/b/c",
            "../../etc/passwd",
            "/absolute/path",
            "C:\\Windows\\System32",
            "..",
            "",
            "n\u{00e9}sted/na\u{0301}me",
        ] {
            let sanitized = sanitize_filename(name);
            let path = PathBuf::from(&sanitized);
            assert_eq!(
                path.components().count(),
                1,
                "{name:?} -> {sanitized:?} is not a single component"
            );
        }
    }

    // ── request_to_http_content ──────────────────────────────────────────────

    #[test]
    fn a_bare_request_becomes_a_method_and_url_line() {
        let content = request_to_http_content(&request(HttpMethod::Get, "https://x.test")).unwrap();
        assert_eq!(content, "GET https://x.test\n");
    }

    #[test]
    fn the_name_and_description_become_leading_annotations() {
        let mut req = request(HttpMethod::Post, "https://x.test/u");
        req.meta.name = Some("Create user".to_string());
        req.meta.description = Some("first line\nsecond line".to_string());
        let content = request_to_http_content(&req).unwrap();
        assert!(content.starts_with("# @name Create user\n"));
        assert!(
            content.contains("# @description first line\n"),
            "only the first line of a description may be emitted, or the \
             continuation lines become stray content: {content:?}"
        );
        assert!(!content.contains("second line"));
        assert!(content.contains("\n\nPOST https://x.test/u\n"));
    }

    #[test]
    fn only_enabled_headers_are_written() {
        let mut req = request(HttpMethod::Get, "https://x.test");
        req.headers = vec![header("X-On", "1", true), header("X-Off", "2", false)];
        let content = request_to_http_content(&req).unwrap();
        assert!(content.contains("X-On: 1\n"));
        assert!(!content.contains("X-Off"));
    }

    #[test]
    fn a_body_is_separated_by_a_blank_line_and_newline_terminated() {
        let mut req = request(HttpMethod::Post, "https://x.test");
        req.body = Some("{\"a\":1}".to_string());
        let content = request_to_http_content(&req).unwrap();
        assert!(content.ends_with("\n\n{\"a\":1}\n"), "got {content:?}");
    }

    #[test]
    fn a_body_that_already_ends_in_a_newline_is_not_double_terminated() {
        let mut req = request(HttpMethod::Post, "https://x.test");
        req.body = Some("line\n".to_string());
        let content = request_to_http_content(&req).unwrap();
        assert!(content.ends_with("line\n"));
        assert!(!content.ends_with("line\n\n"));
    }

    #[test]
    fn generated_content_round_trips_back_through_the_http_parser() {
        // The whole point of this function is to produce a file the app can
        // re-open; anything it emits must parse back to the same request.
        let mut req = request(HttpMethod::Put, "https://x.test/日本語?q=1");
        req.meta.name = Some("Ünïcode request".to_string());
        req.headers = vec![header("Content-Type", "application/json", true)];
        req.body = Some("{\"k\":\"vé\"}".to_string());

        let content = request_to_http_content(&req).unwrap();
        let parsed = http_parser::parse(&content).expect("generated content must re-parse");
        let back = parsed.first().expect("one request");
        assert_eq!(back.method, HttpMethod::Put);
        assert_eq!(back.url, "https://x.test/日本語?q=1");
        assert_eq!(back.meta.name.as_deref(), Some("Ünïcode request"));
        assert_eq!(back.body.as_deref(), Some("{\"k\":\"vé\"}"));
    }
}

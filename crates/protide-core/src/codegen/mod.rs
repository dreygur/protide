//! Code generation module
//!
//! Generates client code from HTTP requests in various languages.

mod curl;
mod go;
mod javascript;
mod python;
mod rust;

use curl::generate_curl;
use go::generate_go;
use javascript::generate_javascript;
use python::generate_python;
use rust::generate_rust;

/// Supported code generation languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Curl,
    Python,
    JavaScript,
    Go,
    Rust,
}

impl Language {
    pub fn name(&self) -> &'static str {
        match self {
            Language::Curl => "cURL",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::Go => "Go",
            Language::Rust => "Rust",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Language::Curl => "sh",
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::Go => "go",
            Language::Rust => "rs",
        }
    }

    pub fn all() -> &'static [Language] {
        &[
            Language::Curl,
            Language::Python,
            Language::JavaScript,
            Language::Go,
            Language::Rust,
        ]
    }
}

/// Request data for code generation
#[derive(Debug, Clone)]
pub struct CodegenRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    /// Content type implied by the body editor mode, used when no header sets one.
    pub content_type: Option<String>,
}

impl CodegenRequest {
    pub fn new(method: &str, url: &str) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            body: None,
            content_type: None,
        }
    }

    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_body(mut self, body: Option<String>) -> Self {
        self.body = body;
        self
    }

    pub fn with_content_type(mut self, content_type: &str) -> Self {
        self.content_type = Some(content_type.to_string());
        self
    }
}

/// The app sends a body for every method except GET and DELETE.
fn sends_body(method: &str) -> bool {
    !method.eq_ignore_ascii_case("GET") && !method.eq_ignore_ascii_case("DELETE")
}

/// The request exactly as the app would put it on the wire: no body where
/// none is sent, and the body type's Content-Type unless the user set one.
fn as_sent(request: &CodegenRequest) -> CodegenRequest {
    let body = request
        .body
        .clone()
        .filter(|b| !b.trim().is_empty() && sends_body(&request.method));
    let mut headers = request.headers.clone();
    if body.is_some()
        && let Some(content_type) = &request.content_type
        && !headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
    {
        headers.push(("Content-Type".to_string(), content_type.clone()));
    }
    CodegenRequest {
        headers,
        body,
        ..request.clone()
    }
}

/// Generate code for a request in the specified language
pub fn generate(request: &CodegenRequest, language: Language) -> String {
    let request = &as_sent(request);
    match language {
        Language::Curl => generate_curl(request),
        Language::Python => generate_python(request),
        Language::JavaScript => generate_javascript(request),
        Language::Go => generate_go(request),
        Language::Rust => generate_rust(request),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> CodegenRequest {
        CodegenRequest::new("POST", "https://api.example.com/users")
            .with_headers(vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("Authorization".to_string(), "Bearer token123".to_string()),
            ])
            .with_body(Some(
                r#"{"name": "John", "email": "john@example.com"}"#.to_string(),
            ))
    }

    #[test]
    fn test_language_names() {
        assert_eq!(Language::Curl.name(), "cURL");
        assert_eq!(Language::Python.name(), "Python");
        assert_eq!(Language::JavaScript.name(), "JavaScript");
        assert_eq!(Language::Go.name(), "Go");
        assert_eq!(Language::Rust.name(), "Rust");
    }

    #[test]
    fn test_language_extensions() {
        assert_eq!(Language::Curl.extension(), "sh");
        assert_eq!(Language::Python.extension(), "py");
        assert_eq!(Language::JavaScript.extension(), "js");
        assert_eq!(Language::Go.extension(), "go");
        assert_eq!(Language::Rust.extension(), "rs");
    }

    #[test]
    fn test_generate_all_languages() {
        let request = sample_request();
        for lang in Language::all() {
            let code = generate(&request, *lang);
            assert!(
                !code.is_empty(),
                "{} should generate non-empty code",
                lang.name()
            );
        }
    }

    /// Every generator must survive hostile/degenerate input without
    /// panicking: control characters, lone quotes, newlines, NUL, emoji and
    /// an empty method/url/header/body.
    #[test]
    fn test_all_generators_survive_adversarial_input() {
        let hostile = [
            "\"",
            "'",
            "\\",
            "\n\r\t",
            "\u{0}\u{7}\u{1b}[31m",
            "🙈🙉🙊",
            "",
            "{{unclosed",
        ];
        for payload in hostile {
            let request = CodegenRequest::new(payload, payload)
                .with_headers(vec![(payload.to_string(), payload.to_string())])
                .with_body(Some(payload.to_string()));
            for lang in Language::all() {
                let code = generate(&request, *lang);
                assert!(
                    !code.is_empty(),
                    "{} produced empty output for payload {:?}",
                    lang.name(),
                    payload
                );
            }
        }
    }

    /// Non-ASCII text must survive codegen intact rather than being mangled
    /// or dropped by the escaping routines.
    #[test]
    fn test_all_generators_preserve_unicode() {
        let request = CodegenRequest::new("POST", "https://例え.テスト/ユーザー?q=🙂")
            .with_headers(vec![("X-Ünïcödé".to_string(), "välüe-🎉".to_string())])
            .with_body(Some("{\"naïve\": \"café ☕\"}".to_string()));
        for lang in Language::all() {
            let code = generate(&request, *lang);
            assert!(
                code.contains("例え.テスト"),
                "{} dropped unicode host: {}",
                lang.name(),
                code
            );
            assert!(
                code.contains("välüe-🎉"),
                "{} dropped unicode header value: {}",
                lang.name(),
                code
            );
            assert!(
                code.contains("café ☕"),
                "{} dropped unicode body: {}",
                lang.name(),
                code
            );
        }
    }

    /// The app sends no body for GET or DELETE, so no generator may emit one.
    #[test]
    fn test_no_generator_emits_a_body_the_app_does_not_send() {
        for method in ["GET", "DELETE", "get"] {
            let request = CodegenRequest::new(method, "https://example.com")
                .with_body(Some("stale-body-marker".to_string()))
                .with_content_type("application/json");
            for lang in Language::all() {
                let code = generate(&request, *lang);
                assert!(
                    !code.contains("stale-body-marker") && !code.contains("application/json"),
                    "{} emitted a body for {}: {}",
                    lang.name(),
                    method,
                    code
                );
            }
        }
    }

    #[test]
    fn test_all_generators_add_the_body_content_type_when_unset() {
        for ct in ["application/json", "application/xml"] {
            let request = CodegenRequest::new("POST", "https://example.com")
                .with_body(Some("<a/>".to_string()))
                .with_content_type(ct);
            for lang in Language::all() {
                let code = generate(&request, *lang);
                assert!(
                    code.contains("Content-Type") && code.contains(ct),
                    "{} omitted Content-Type {}: {}",
                    lang.name(),
                    ct,
                    code
                );
            }
        }
    }

    #[test]
    fn test_a_user_content_type_wins_over_the_body_type() {
        let request = CodegenRequest::new("PUT", "https://example.com")
            .with_headers(vec![("content-type".to_string(), "text/csv".to_string())])
            .with_body(Some("a,b".to_string()))
            .with_content_type("application/json");
        for lang in Language::all() {
            let code = generate(&request, *lang);
            assert!(
                code.contains("text/csv") && !code.contains("application/json"),
                "{} overrode the user Content-Type: {}",
                lang.name(),
                code
            );
        }
    }

    #[test]
    fn test_no_content_type_is_added_without_a_body() {
        let request = CodegenRequest::new("POST", "https://example.com")
            .with_body(Some("  ".to_string()))
            .with_content_type("application/json");
        for lang in Language::all() {
            let code = generate(&request, *lang);
            assert!(
                !code.contains("application/json"),
                "{} added Content-Type without a body: {}",
                lang.name(),
                code
            );
        }
    }

    /// A body large enough to matter must not be truncated by any generator.
    #[test]
    fn test_all_generators_keep_large_bodies_intact() {
        let body = "x".repeat(200_000);
        let request = CodegenRequest::new("POST", "https://example.com").with_body(Some(body));
        for lang in Language::all() {
            let code = generate(&request, *lang);
            assert!(
                code.contains(&"x".repeat(200_000)),
                "{} truncated a large body",
                lang.name()
            );
        }
    }
}

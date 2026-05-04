//! Code generation module
//!
//! Generates client code from HTTP requests in various languages.

mod curl;
mod go;
mod javascript;
mod python;
mod rust;

pub use curl::generate_curl;
pub use go::generate_go;
pub use javascript::generate_javascript;
pub use python::generate_python;
pub use rust::generate_rust;

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
}

impl CodegenRequest {
    pub fn new(method: &str, url: &str) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            headers: Vec::new(),
            body: None,
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
}

/// Generate code for a request in the specified language
pub fn generate(request: &CodegenRequest, language: Language) -> String {
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
            .with_body(Some(r#"{"name": "John", "email": "john@example.com"}"#.to_string()))
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
            assert!(!code.is_empty(), "{} should generate non-empty code", lang.name());
        }
    }
}

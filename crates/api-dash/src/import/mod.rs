//! Import functionality for various API formats
//!
//! Supports importing from:
//! - cURL commands
//! - Postman Collection v2.1
//! - OpenAPI/Swagger specifications

mod curl;
mod openapi;
mod postman;

pub use curl::parse_curl;
pub use openapi::parse_openapi;
pub use postman::parse_postman;

use http_parser::Request;
use std::path::Path;

/// Supported import formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    Curl,
    Postman,
    OpenApi,
}

impl ImportFormat {
    /// Detect format from file extension
    pub fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "json" => {
                // Could be Postman or OpenAPI - need to inspect content
                None
            }
            "yaml" | "yml" => Some(Self::OpenApi),
            _ => None,
        }
    }

    /// Detect format from content
    pub fn detect(content: &str) -> Option<Self> {
        let trimmed = content.trim();

        // cURL command
        if trimmed.starts_with("curl ") || trimmed.starts_with("curl\t") {
            return Some(Self::Curl);
        }

        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
            // Postman Collection v2.1
            if json.get("info").and_then(|i| i.get("schema")).is_some() {
                return Some(Self::Postman);
            }
            // OpenAPI 3.x
            if json.get("openapi").is_some() {
                return Some(Self::OpenApi);
            }
            // Swagger 2.0
            if json.get("swagger").is_some() {
                return Some(Self::OpenApi);
            }
        }

        // Try YAML (OpenAPI)
        if trimmed.contains("openapi:") || trimmed.contains("swagger:") {
            return Some(Self::OpenApi);
        }

        None
    }
}

/// Import result containing parsed requests
#[derive(Debug)]
pub struct ImportResult {
    /// Collection name (if available)
    pub name: Option<String>,
    /// Parsed requests
    pub requests: Vec<Request>,
    /// Any warnings during import
    pub warnings: Vec<String>,
}

impl ImportResult {
    pub fn new() -> Self {
        Self {
            name: None,
            requests: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn add_request(&mut self, request: Request) {
        self.requests.push(request);
    }

    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

impl Default for ImportResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Import from any supported format (auto-detect)
pub fn import(content: &str) -> Result<ImportResult, String> {
    let format = ImportFormat::detect(content)
        .ok_or_else(|| "Could not detect import format".to_string())?;

    match format {
        ImportFormat::Curl => parse_curl(content),
        ImportFormat::Postman => parse_postman(content),
        ImportFormat::OpenApi => parse_openapi(content),
    }
}

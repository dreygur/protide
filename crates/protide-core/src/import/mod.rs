//! Import functionality for various API formats
//!
//! Supports importing from:
//! - cURL commands
//! - Postman Collection v2.1
//! - OpenAPI/Swagger specifications
//! - Bruno .bru files

mod bruno;
mod curl;
mod openapi;
mod postman;

pub use bruno::parse_bruno;
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
    Bruno,
}

impl ImportFormat {
    /// Detect format from file extension
    pub fn from_extension(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "bru" => Some(Self::Bruno),
            "json" => None, // Could be Postman or OpenAPI - inspect content
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

        // Bruno .bru format (has "meta {" block)
        if trimmed.contains("meta {") && (trimmed.contains("get {") || trimmed.contains("post {")
            || trimmed.contains("put {") || trimmed.contains("delete {")
            || trimmed.contains("patch {")) {
            return Some(Self::Bruno);
        }

        // Try to parse as JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if json.get("info").and_then(|i| i.get("schema")).is_some() {
                return Some(Self::Postman);
            }
            if json.get("openapi").is_some() {
                return Some(Self::OpenApi);
            }
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
    /// Optional subfolder for each request (same length as requests)
    pub request_folders: Vec<Option<String>>,
    /// Any warnings during import
    pub warnings: Vec<String>,
}

impl ImportResult {
    pub fn new() -> Self {
        Self {
            name: None,
            requests: Vec::new(),
            request_folders: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn add_request(&mut self, request: Request) {
        self.requests.push(request);
        self.request_folders.push(None);
    }

    pub fn add_request_in_folder(&mut self, request: Request, folder: Option<String>) {
        self.requests.push(request);
        self.request_folders.push(folder);
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
        .ok_or_else(|| "Could not detect import format. Supported: cURL, Postman, OpenAPI, Bruno .bru".to_string())?;

    match format {
        ImportFormat::Curl => parse_curl(content),
        ImportFormat::Postman => parse_postman(content),
        ImportFormat::OpenApi => parse_openapi(content),
        ImportFormat::Bruno => parse_bruno(content),
    }
}

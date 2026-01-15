//! Request panel types and enums
//!
//! Contains shared types used by the request panel.

use std::path::PathBuf;

/// Key-value pair for headers, params, etc.
#[derive(Clone, Default)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

/// Form field type (text or file)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FormFieldType {
    #[default]
    Text,
    File,
}

/// Form field that can be text or file
#[derive(Clone, Default)]
pub struct FormField {
    pub key: String,
    pub value: String,           // Text value or display name for file
    pub field_type: FormFieldType,
    pub file_path: Option<PathBuf>, // Path to file when field_type is File
    pub enabled: bool,
}

/// Target for text editing
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditTarget {
    #[allow(dead_code)] // Used in match arms for completeness
    Url,
    ParamKey(usize),
    ParamValue(usize),
    HeaderKey(usize),
    HeaderValue(usize),
    #[allow(dead_code)]
    Body,
    BearerToken,
    BasicUsername,
    BasicPassword,
    ApiKeyName,
    ApiKeyValue,
    FormKey(usize),
    FormValue(usize),
}

/// Authentication type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AuthType {
    #[default]
    None,
    Bearer,
    Basic,
    ApiKey,
}

/// API key location
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApiKeyLocation {
    #[default]
    Header,
    QueryParam,
}

/// HTTP request method
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "PATCH" => Some(HttpMethod::Patch),
            "DELETE" => Some(HttpMethod::Delete),
            _ => None,
        }
    }

    pub fn all() -> &'static [HttpMethod] {
        &[
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Patch,
            HttpMethod::Delete,
        ]
    }
}

/// Body content type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyType {
    #[default]
    Json,
    Raw,
    Form,
}

/// Request mode (HTTP or GraphQL)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestMode {
    #[default]
    Http,
    GraphQL,
}

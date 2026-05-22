use super::*;

/// Response data from an HTTP request
#[derive(Clone, Default)]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub time: Duration,
    pub size: usize,
}

impl ResponseData {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn is_error(&self) -> bool {
        self.status >= 400
    }
}

/// Parsed cookie from Set-Cookie header
#[derive(Clone, Debug)]
pub struct ParsedCookie {
    pub name: String,
    pub value: String,
    pub path: Option<String>,
    pub domain: Option<String>,
    pub expires: Option<String>,
    pub secure: bool,
    pub http_only: bool,
}

impl ParsedCookie {
    /// Parse a Set-Cookie header value
    pub fn parse(header_value: &str) -> Option<Self> {
        let mut parts = header_value.split(';');
        let name_value = parts.next()?.trim();
        let mut split = name_value.splitn(2, '=');
        let name = split.next()?.trim().to_string();
        let value = split.next().unwrap_or("").trim().to_string();

        if name.is_empty() {
            return None;
        }

        let mut cookie = ParsedCookie {
            name,
            value,
            path: None,
            domain: None,
            expires: None,
            secure: false,
            http_only: false,
        };

        for part in parts {
            let part = part.trim();
            let lower = part.to_lowercase();
            if lower == "secure" {
                cookie.secure = true;
            } else if lower == "httponly" {
                cookie.http_only = true;
            } else if let Some(val) = part.strip_prefix("Path=").or_else(|| part.strip_prefix("path=")) {
                cookie.path = Some(val.to_string());
            } else if let Some(val) = part.strip_prefix("Domain=").or_else(|| part.strip_prefix("domain=")) {
                cookie.domain = Some(val.to_string());
            } else if let Some(val) = part.strip_prefix("Expires=").or_else(|| part.strip_prefix("expires=")) {
                cookie.expires = Some(val.to_string());
            }
        }
        Some(cookie)
    }
}

/// Active text selection within the header value column
#[derive(Debug, Clone, Copy)]
pub(crate) struct HdrSel {
    pub row: usize,
    pub range: (usize, usize), // (anchor_byte, head_byte) - un-normalized
    pub selecting: bool,
}

/// Copy feedback type
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyFeedback {
    Body,
    Headers,
    HdrVal,
}

pub(super) fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub(super) fn truncate_error(error: &str) -> String {
    if error.len() > 40 {
        format!("{}...", &error[..37])
    } else {
        error.to_string()
    }
}

pub(super) fn status_description(status: u16) -> Option<&'static str> {
    match status {
        // 1xx Informational
        100 => Some("Continue - Server received request headers"),
        101 => Some("Switching Protocols"),
        102 => Some("Processing - Server is processing the request"),
        103 => Some("Early Hints"),

        // 2xx Success
        200 => Some("Request succeeded"),
        201 => Some("Resource created successfully"),
        202 => Some("Request accepted for processing"),
        203 => Some("Non-authoritative information"),
        204 => Some("No content to return"),
        205 => Some("Reset content"),
        206 => Some("Partial content delivered"),
        207 => Some("Multi-status response"),
        208 => Some("Already reported"),
        226 => Some("IM Used"),

        // 3xx Redirection
        300 => Some("Multiple choices available"),
        301 => Some("Resource moved permanently"),
        302 => Some("Resource found at different URI"),
        303 => Some("See other resource"),
        304 => Some("Resource not modified"),
        305 => Some("Use proxy"),
        307 => Some("Temporary redirect"),
        308 => Some("Permanent redirect"),

        // 4xx Client Errors
        400 => Some("Bad request syntax or invalid"),
        401 => Some("Authentication required"),
        402 => Some("Payment required"),
        403 => Some("Access forbidden"),
        404 => Some("Resource not found"),
        405 => Some("Method not allowed"),
        406 => Some("Not acceptable format"),
        407 => Some("Proxy authentication required"),
        408 => Some("Request timeout"),
        409 => Some("Conflict with current state"),
        410 => Some("Resource no longer available"),
        411 => Some("Length required"),
        412 => Some("Precondition failed"),
        413 => Some("Payload too large"),
        414 => Some("URI too long"),
        415 => Some("Unsupported media type"),
        416 => Some("Range not satisfiable"),
        417 => Some("Expectation failed"),
        418 => Some("I'm a teapot"),
        421 => Some("Misdirected request"),
        422 => Some("Unprocessable entity"),
        423 => Some("Resource is locked"),
        424 => Some("Failed dependency"),
        425 => Some("Too early"),
        426 => Some("Upgrade required"),
        428 => Some("Precondition required"),
        429 => Some("Too many requests"),
        431 => Some("Request header fields too large"),
        451 => Some("Unavailable for legal reasons"),

        // 5xx Server Errors
        500 => Some("Internal server error"),
        501 => Some("Not implemented"),
        502 => Some("Bad gateway"),
        503 => Some("Service unavailable"),
        504 => Some("Gateway timeout"),
        505 => Some("HTTP version not supported"),
        506 => Some("Variant also negotiates"),
        507 => Some("Insufficient storage"),
        508 => Some("Loop detected"),
        510 => Some("Not extended"),
        511 => Some("Network authentication required"),

        _ => None,
    }
}

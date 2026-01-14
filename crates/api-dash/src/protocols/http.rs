//! HTTP client implementation using reqwest

#![allow(dead_code)]

use anyhow::Result;
use http_parser::Request;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;
use std::time::Instant;

use crate::models::{Environment, Response};

/// HTTP client for making requests
pub struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .build()?;
        Ok(Self { client })
    }

    /// Execute an HTTP request
    pub async fn execute(
        &self,
        request: &Request,
        env: Option<&Environment>,
    ) -> Result<Response> {
        let start = Instant::now();

        // Substitute environment variables in URL
        let url = if let Some(env) = env {
            env.substitute(&request.url)
        } else {
            request.url.clone()
        };

        // Build request
        let method = reqwest::Method::from_str(request.method.as_str())
            .unwrap_or(reqwest::Method::GET);

        let mut req_builder = self.client.request(method, &url);

        // Add headers
        let mut headers = HeaderMap::new();
        for header in &request.headers {
            if !header.enabled {
                continue;
            }
            let key = if let Some(env) = env {
                env.substitute(&header.key)
            } else {
                header.key.clone()
            };
            let value = if let Some(env) = env {
                env.substitute(&header.value)
            } else {
                header.value.clone()
            };

            if let (Ok(name), Ok(val)) = (
                HeaderName::from_str(&key),
                HeaderValue::from_str(&value),
            ) {
                headers.insert(name, val);
            }
        }
        req_builder = req_builder.headers(headers);

        // Add body
        if let Some(body) = &request.body {
            let body = if let Some(env) = env {
                env.substitute(body)
            } else {
                body.clone()
            };
            req_builder = req_builder.body(body);
        }

        // Send request
        let res = req_builder.send().await?;

        let elapsed = start.elapsed();
        let status = res.status().as_u16();

        // Collect response headers
        let headers: Vec<http_parser::KeyValue> = res
            .headers()
            .iter()
            .map(|(k, v)| {
                http_parser::KeyValue::new(
                    k.as_str(),
                    v.to_str().unwrap_or(""),
                )
            })
            .collect();

        // Get body
        let body = res.text().await?;
        let size = body.len();

        Ok(Response {
            status,
            status_text: status_text(status).to_string(),
            headers,
            body,
            time: elapsed,
            size,
            protocol: http_parser::Protocol::Http,
        })
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create HTTP client")
    }
}

fn status_text(status: u16) -> &'static str {
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

//! Parser for .http files
//!
//! Parses tokenized .http content into structured Request objects.

use crate::ast::*;
use crate::lexer::{Lexer, ScriptType, Token, TokenSpan};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token at line {line}: expected {expected}, got {got}")]
    UnexpectedToken {
        line: usize,
        expected: String,
        got: String,
    },

    #[error("Invalid HTTP method at line {line}: {method}")]
    InvalidMethod { line: usize, method: String },

    #[error("Missing URL at line {line}")]
    MissingUrl { line: usize },

    #[error("Invalid URL at line {line}: {url}")]
    InvalidUrl { line: usize, url: String },

    #[error("Invalid protocol at line {line}: {protocol}")]
    InvalidProtocol { line: usize, protocol: String },
}

/// Parser for .http files
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: TokenSpan,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        Self {
            lexer,
            current_token,
        }
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    /// Line number the currently-buffered token was actually read from.
    fn line_number(&self) -> usize {
        self.current_token.line
    }

    /// Parse the entire file into a list of requests
    pub fn parse(&mut self) -> Result<Vec<Request>, ParseError> {
        let mut requests = Vec::new();

        // Skip leading empty lines and comments
        self.skip_whitespace();

        while self.current_token.token != Token::Eof {
            if let Some(request) = self.parse_request()? {
                requests.push(request);
            }
            self.skip_whitespace();
        }

        Ok(requests)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match &self.current_token.token {
                Token::EmptyLine | Token::Comment(_) => self.advance(),
                Token::RequestSeparator => self.advance(),
                _ => break,
            }
        }
    }

    /// Parse a single request
    fn parse_request(&mut self) -> Result<Option<Request>, ParseError> {
        let mut meta = RequestMeta::default();
        let mut scripts = Scripts::default();

        // Parse annotations, plain comments, and empty lines before the request line
        loop {
            match &self.current_token.token {
                Token::Annotation(key, value) => {
                    let (k, v) = (key.clone(), value.clone());
                    self.parse_annotation(&mut meta, k, v)?;
                    self.advance();
                }
                Token::SetVariable(name, expr) => {
                    meta.variable_extractions.push(VariableExtraction {
                        name: name.clone(),
                        expression: expr.clone(),
                    });
                    self.advance();
                }
                Token::Comment(_) | Token::EmptyLine => {
                    self.advance();
                }
                _ => break,
            }
        }

        // Expect HTTP method
        let (method, start_line) = match &self.current_token.token {
            Token::Method(m) => {
                let method = HttpMethod::from_str(m).ok_or_else(|| ParseError::InvalidMethod {
                    line: self.line_number(),
                    method: m.clone(),
                })?;
                let start_line = self.line_number();
                self.advance();
                (method, start_line)
            }
            Token::Eof => return Ok(None),
            Token::RequestSeparator => {
                self.advance();
                return self.parse_request();
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    line: self.line_number(),
                    expected: "HTTP method".to_string(),
                    got: format!("{:?}", self.current_token.token),
                });
            }
        };

        // Parse URL (might be on same line as method or next line)
        let url = match &self.current_token.token {
            Token::Url(u) => {
                let url = u.clone();
                self.advance();
                url
            }
            Token::Header(_, _) | Token::EmptyLine | Token::Eof => {
                // URL might have been part of the method line
                return Err(ParseError::MissingUrl {
                    line: self.line_number(),
                });
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    line: self.line_number(),
                    expected: "URL".to_string(),
                    got: format!("{:?}", self.current_token.token),
                });
            }
        };

        // Parse headers. A commented-out header line (`# Key: Value`) that
        // looks like a real header is treated as a disabled header, so the
        // enabled/disabled state can round-trip through save/load. Only
        // comments immediately within the header block are considered here,
        // so ordinary comments elsewhere in the file are unaffected.
        let mut headers = Vec::new();
        loop {
            match &self.current_token.token {
                Token::Header(key, value) => {
                    headers.push(KeyValue::new(key.clone(), value.clone()));
                    self.advance();
                }
                Token::Comment(text) => match parse_disabled_header(text) {
                    Some((key, value)) => {
                        headers.push(KeyValue { key, value, enabled: false });
                        self.advance();
                    }
                    None => break,
                },
                _ => break,
            }
        }

        // Skip empty line before body
        self.skip_empty_lines();

        // Parse body
        let mut body_lines = Vec::new();
        while let Token::Body(line) = &self.current_token.token {
            body_lines.push(line.clone());
            self.advance();
        }

        let body = if body_lines.is_empty() {
            None
        } else {
            // Trim trailing empty lines from body
            while body_lines.last().map(|s| s.trim().is_empty()).unwrap_or(false) {
                body_lines.pop();
            }
            if body_lines.is_empty() {
                None
            } else {
                Some(body_lines.join("\n"))
            }
        };

        // Parse post-body annotations and scripts
        self.skip_empty_lines();

        loop {
            match &self.current_token.token {
                Token::Annotation(key, value) => {
                    self.parse_annotation(&mut meta, key.clone(), value.clone())?;
                    self.advance();
                }
                Token::SetVariable(name, expr) => {
                    meta.variable_extractions.push(VariableExtraction {
                        name: name.clone(),
                        expression: expr.clone(),
                    });
                    self.advance();
                }
                Token::ScriptMarker(script_type) => {
                    let script_type = *script_type;
                    self.advance();
                    let script = self.parse_script_block();
                    match script_type {
                        ScriptType::PreScript => scripts.pre_script = Some(script),
                        ScriptType::PostScript => scripts.post_script = Some(script),
                        ScriptType::Tests => scripts.tests = Some(script),
                    }
                }
                Token::EmptyLine => self.advance(),
                Token::Comment(_) => self.advance(),
                _ => break,
            }
        }

        Ok(Some(Request {
            meta,
            method,
            url,
            headers,
            body,
            scripts,
            line: start_line,
        }))
    }

    fn skip_empty_lines(&mut self) {
        while matches!(self.current_token.token, Token::EmptyLine) {
            self.advance();
        }
    }

    fn parse_annotation(
        &mut self,
        meta: &mut RequestMeta,
        key: String,
        value: Option<String>,
    ) -> Result<(), ParseError> {
        match key.as_str() {
            "name" => meta.name = value,
            "description" => meta.description = value,
            "protocol" => {
                if let Some(v) = value {
                    meta.protocol = Some(parse_protocol(&v).ok_or_else(|| {
                        ParseError::InvalidProtocol {
                            line: self.line_number(),
                            protocol: v,
                        }
                    })?);
                }
            }
            "proto" => meta.proto_path = value,
            "depends" => {
                if let Some(v) = value {
                    meta.depends.extend(v.split(',').map(|s| s.trim().to_string()));
                }
            }
            _ => {} // Ignore unknown annotations
        }
        Ok(())
    }

    fn parse_script_block(&mut self) -> String {
        let mut lines = Vec::new();
        while let Token::ScriptLine(line) = &self.current_token.token {
            lines.push(line.clone());
            self.advance();
        }
        lines.join("\n")
    }
}

fn parse_protocol(s: &str) -> Option<Protocol> {
    match s.to_lowercase().as_str() {
        "http" | "rest" => Some(Protocol::Http),
        "graphql" | "gql" => Some(Protocol::GraphQL),
        "websocket" | "ws" => Some(Protocol::WebSocket),
        "grpc" => Some(Protocol::Grpc),
        "socketio" | "socket.io" => Some(Protocol::SocketIO),
        "trpc" => Some(Protocol::Trpc),
        _ => None,
    }
}

/// Parse a comment's text (already stripped of the leading `#`) as a
/// disabled header line (`Key: Value`), using the same key validity rules
/// as the lexer's real `Header` token so a disabled header round-trips
/// exactly like an enabled one. Returns `None` for comments that don't
/// look like a header, so freeform comments are left untouched.
fn parse_disabled_header(text: &str) -> Option<(String, String)> {
    let colon_pos = text.find(':')?;
    let key = text[..colon_pos].trim();
    let value = text[colon_pos + 1..].trim();

    if key.is_empty() || key.contains(' ') || !key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return None;
    }

    Some((key.to_string(), value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(content: &str) -> Result<Vec<Request>, ParseError> {
        let lexer = Lexer::new(content);
        let mut parser = Parser::new(lexer);
        parser.parse()
    }

    #[test]
    fn test_parse_simple_get() {
        let content = r#"GET https://api.example.com/users"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, HttpMethod::Get);
        assert_eq!(requests[0].url, "https://api.example.com/users");
    }

    #[test]
    fn test_parse_with_headers() {
        let content = r#"
GET https://api.example.com/users
Authorization: Bearer token123
Content-Type: application/json
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].headers.len(), 2);
        assert_eq!(requests[0].get_header("Authorization"), Some("Bearer token123"));
    }

    #[test]
    fn test_parse_with_body() {
        let content = r#"
POST https://api.example.com/users
Content-Type: application/json

{"name": "John", "email": "john@example.com"}
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, HttpMethod::Post);
        assert!(requests[0].body.is_some());
        assert!(requests[0].body.as_ref().unwrap().contains("John"));
    }

    #[test]
    fn test_parse_with_annotations() {
        let content = r#"
### Create User
# @name create-user
# @description Creates a new user account

POST https://api.example.com/users
Content-Type: application/json

{"name": "John"}
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].meta.name, Some("create-user".to_string()));
        assert!(requests[0].meta.description.is_some());
    }

    #[test]
    fn test_parse_multiple_requests() {
        let content = r#"
### Get Users
GET https://api.example.com/users

###

### Create User
POST https://api.example.com/users
Content-Type: application/json

{"name": "John"}
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, HttpMethod::Get);
        assert_eq!(requests[1].method, HttpMethod::Post);
    }

    #[test]
    fn test_request_line_numbers_point_at_method_line() {
        // Realistic multi-request document with ### separators, blank lines,
        // and annotations preceding each request. `Request.line` must point
        // at the actual method-keyword line, not a preceding separator or
        // blank line (see http-parser lexer/parser TokenSpan fix).
        let content = "### Get Users\n\
GET https://api.example.com/users\n\
\n\
###\n\
\n\
### Create User\n\
# @name create-user\n\
POST https://api.example.com/users\n\
Content-Type: application/json\n\
\n\
{\"name\": \"John\"}\n\
\n\
### Delete User\n\
\n\
DELETE https://api.example.com/users/1\n";

        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 3);

        assert_eq!(requests[0].method, HttpMethod::Get);
        assert_eq!(requests[0].line, 2, "GET is on line 2");

        assert_eq!(requests[1].method, HttpMethod::Post);
        assert_eq!(requests[1].line, 8, "POST is on line 8, after the @name annotation");

        assert_eq!(requests[2].method, HttpMethod::Delete);
        assert_eq!(requests[2].line, 15, "DELETE is on line 15, after a blank line");
    }

    #[test]
    fn test_parse_graphql() {
        let content = r#"
### Get User by ID
# @protocol graphql

POST https://api.example.com/graphql
Content-Type: application/json

{"query": "query { user(id: 1) { name } }"}
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].protocol(), Protocol::GraphQL);
    }

    #[test]
    fn test_parse_with_variable_extraction() {
        let content = r#"
### Login
# @name login

POST https://api.example.com/auth/login
Content-Type: application/json

{"email": "test@example.com", "password": "secret"}

# @set access_token = $.token
# @set user_id = $.user.id
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].meta.variable_extractions.len(), 2);
        assert_eq!(requests[0].meta.variable_extractions[0].name, "access_token");
    }

    #[test]
    fn parse_e2e_http_api_tests() {
        let content = include_str!("../../../e2e/http/http-api-tests.http");
        let r = parse(content).expect("http-api-tests.http failed to parse");
        assert!(r.len() >= 13, "expected ≥13 requests, got {}", r.len());
    }

    #[test]
    fn parse_e2e_http_scripting() {
        let content = include_str!("../../../e2e/http/http-scripting.http");
        let r = parse(content).expect("http-scripting.http failed to parse");
        assert!(r.len() >= 10);
    }

    #[test]
    fn parse_e2e_graphql() {
        let content = include_str!("../../../e2e/graphql/graphql-tests.http");
        let r = parse(content).expect("graphql-tests.http failed to parse");
        assert!(r.iter().all(|req| req.protocol() == Protocol::GraphQL
            || req.meta.protocol == Some(Protocol::GraphQL)));
    }

    #[test]
    fn parse_e2e_websocket() {
        let content = include_str!("../../../e2e/websocket/websocket-echo.http");
        let r = parse(content).expect("websocket-echo.http failed to parse");
        assert!(r.len() >= 5);
    }

    #[test]
    fn parse_e2e_socketio() {
        let content = include_str!("../../../e2e/socketio/socketio-echo.http");
        let r = parse(content).expect("socketio-echo.http failed to parse");
        assert!(r.len() >= 5);
    }

    #[test]
    fn parse_e2e_grpc() {
        let content = include_str!("../../../e2e/grpc/grpc-employee.http");
        let r = parse(content).expect("grpc-employee.http failed to parse");
        assert!(r.len() >= 5);
    }

    #[test]
    fn parse_e2e_trpc() {
        let content = include_str!("../../../e2e/trpc/trpc-example.http");
        let r = parse(content).expect("trpc-example.http failed to parse");
        assert!(r.len() >= 7);
    }

    #[test]
    fn test_parse_disabled_header() {
        let content = r#"
GET https://api.example.com/users
Authorization: Bearer token123
# X-Disabled: should-not-be-sent
Content-Type: application/json
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 1);
        let headers = &requests[0].headers;
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[0].key, "Authorization");
        assert!(headers[0].enabled);
        assert_eq!(headers[1].key, "X-Disabled");
        assert_eq!(headers[1].value, "should-not-be-sent");
        assert!(!headers[1].enabled);
        assert_eq!(headers[2].key, "Content-Type");
        assert!(headers[2].enabled);
    }

    #[test]
    fn test_parse_header_block_comment_not_mistaken_for_header() {
        // A comment in the header block that doesn't look like `Key: Value`
        // (no colon, or an invalid key) must stop header parsing exactly as
        // before, rather than being swallowed as a disabled header.
        let content = r#"
GET https://api.example.com/users
Authorization: Bearer token123
# just a note, no colon shape
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests[0].headers.len(), 1);
    }
}

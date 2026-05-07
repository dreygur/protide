//! Parser for .http files
//!
//! Parses tokenized .http content into structured Request objects.

use crate::ast::*;
use crate::lexer::{Lexer, ScriptType, Token};
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
    current_token: Token,
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

    fn line_number(&self) -> usize {
        self.lexer.line_number()
    }

    /// Parse the entire file into a list of requests
    pub fn parse(&mut self) -> Result<Vec<Request>, ParseError> {
        let mut requests = Vec::new();

        // Skip leading empty lines and comments
        self.skip_whitespace();

        while self.current_token != Token::Eof {
            if let Some(request) = self.parse_request()? {
                requests.push(request);
            }
            self.skip_whitespace();
        }

        Ok(requests)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match &self.current_token {
                Token::EmptyLine | Token::Comment(_) => self.advance(),
                Token::RequestSeparator => self.advance(),
                _ => break,
            }
        }
    }

    /// Parse a single request
    fn parse_request(&mut self) -> Result<Option<Request>, ParseError> {
        let start_line = self.line_number();
        let mut meta = RequestMeta::default();
        let mut scripts = Scripts::default();

        // Parse annotations, plain comments, and empty lines before the request line
        loop {
            match &self.current_token {
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
        let method = match &self.current_token {
            Token::Method(m) => {
                let method = HttpMethod::from_str(m).ok_or_else(|| ParseError::InvalidMethod {
                    line: self.line_number(),
                    method: m.clone(),
                })?;
                self.advance();
                method
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
                    got: format!("{:?}", self.current_token),
                });
            }
        };

        // Parse URL (might be on same line as method or next line)
        let url = match &self.current_token {
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
                    got: format!("{:?}", self.current_token),
                });
            }
        };

        // Parse headers
        let mut headers = Vec::new();
        while let Token::Header(key, value) = &self.current_token {
            headers.push(KeyValue::new(key.clone(), value.clone()));
            self.advance();
        }

        // Skip empty line before body
        self.skip_empty_lines();

        // Parse body
        let mut body_lines = Vec::new();
        while let Token::Body(line) = &self.current_token {
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
            match &self.current_token {
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
        while matches!(self.current_token, Token::EmptyLine) {
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
        while let Token::ScriptLine(line) = &self.current_token {
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
}

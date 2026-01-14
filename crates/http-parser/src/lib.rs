//! # http-parser
//!
//! A parser for .http files with extended protocol support for:
//! - HTTP/REST requests
//! - GraphQL queries
//! - WebSocket connections
//! - gRPC calls
//! - Socket.IO events
//! - tRPC procedures
//!
//! ## File Format
//!
//! ```http
//! ### Request name
//! # @name my-request
//! # @protocol http
//!
//! GET https://api.example.com/users
//! Authorization: Bearer {{token}}
//! Content-Type: application/json
//!
//! {"key": "value"}
//!
//! # @pre-script
//! // JavaScript code
//!
//! # @post-script
//! // JavaScript code
//!
//! # @tests
//! expect(response.status).toBe(200);
//! ```

mod ast;
mod lexer;
mod parser;

pub use ast::*;
pub use lexer::Lexer;
pub use parser::{Parser, ParseError};

/// Parse a .http file content into a list of requests
pub fn parse(content: &str) -> Result<Vec<Request>, ParseError> {
    let lexer = Lexer::new(content);
    let mut parser = Parser::new(lexer);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get() {
        let content = r#"
### Get users
GET https://api.example.com/users
Authorization: Bearer token123
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, HttpMethod::Get);
        assert_eq!(requests[0].url, "https://api.example.com/users");
    }

    #[test]
    fn test_parse_post_with_body() {
        let content = r#"
### Create user
POST https://api.example.com/users
Content-Type: application/json

{"name": "John", "email": "john@example.com"}
"#;
        let requests = parse(content).unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, HttpMethod::Post);
        assert!(requests[0].body.is_some());
    }
}

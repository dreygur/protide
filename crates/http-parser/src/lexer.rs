//! Lexer for .http files
//!
//! Tokenizes .http file content into a stream of tokens for the parser.

/// Schemes that make a line unambiguously a URL rather than a header.
const URL_SCHEMES: [&str; 5] = ["http://", "https://", "ws://", "wss://", "grpc://"];

/// Token types for the .http file format
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Request separator (###)
    RequestSeparator,
    /// Comment line starting with # (not an annotation)
    Comment(String),
    /// Annotation like @name, @protocol, etc.
    Annotation(String, Option<String>),
    /// HTTP method (GET, POST, etc.)
    Method(String),
    /// URL
    Url(String),
    /// Header line (Key: Value)
    Header(String, String),
    /// Body content line
    Body(String),
    /// Empty line
    EmptyLine,
    /// Script block marker (# @pre-script, # @post-script, # @tests)
    ScriptMarker(ScriptType),
    /// Script content line
    ScriptLine(String),
    /// Variable extraction (# @set name = expression)
    SetVariable(String, String),
    /// End of file
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptType {
    PreScript,
    PostScript,
    Tests,
}

/// A token paired with the 1-indexed source line it was produced from.
///
/// Tokens are attached to their line number at the moment they're emitted by
/// the lexer, rather than derived later from the lexer's current cursor
/// position. This matters because the parser (see `Parser::new`) eagerly
/// pre-fetches one token of lookahead, so by the time a token is examined the
/// lexer's cursor has already moved past it - querying "current" line lazily
/// would report the position of the *next* unread token, not the token in
/// hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenSpan {
    pub token: Token,
    pub line: usize,
}

/// Lexer for .http files
pub struct Lexer<'a> {
    content: &'a str,
    lines: Vec<&'a str>,
    current_line: usize,
    in_script: Option<ScriptType>,
    in_body: bool,
    /// Pending token to return on next call (used when method and URL are on same line)
    pending_token: Option<TokenSpan>,
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a str) -> Self {
        // A UTF-8 BOM is not whitespace, so left in place it would glue itself
        // to the first token (typically the method keyword) and make an
        // otherwise valid file unparseable. Editors on Windows write it freely.
        let content = content.strip_prefix('\u{feff}').unwrap_or(content);
        let lines: Vec<&str> = content.lines().collect();
        Self {
            content,
            lines,
            current_line: 0,
            in_script: None,
            in_body: false,
            pending_token: None,
        }
    }

    /// Peek at the next token without consuming it
    pub fn peek(&self) -> TokenSpan {
        let mut clone = Self {
            content: self.content,
            lines: self.lines.clone(),
            current_line: self.current_line,
            in_script: self.in_script,
            in_body: self.in_body,
            pending_token: self.pending_token.clone(),
        };
        clone.next_token()
    }

    /// Get the next token, tagged with the 1-indexed source line it came from.
    pub fn next_token(&mut self) -> TokenSpan {
        // Check for pending token first
        if let Some(token) = self.pending_token.take() {
            return token;
        }

        if self.current_line >= self.lines.len() {
            return TokenSpan {
                token: Token::Eof,
                line: self.current_line + 1,
            };
        }

        // Capture the line number for the line about to be consumed, before
        // advancing the cursor - this is the line the produced token(s) belong to.
        let line_num = self.current_line + 1;
        let line = self.lines[self.current_line];
        self.current_line += 1;

        // Handle script blocks
        if let Some(_script_type) = self.in_script {
            // Check if we're exiting the script block
            if line.trim().starts_with("###") || line.trim().starts_with("# @") {
                self.in_script = None;
                self.current_line -= 1; // Re-process this line
                return self.next_token();
            }
            return TokenSpan {
                token: Token::ScriptLine(line.to_string()),
                line: line_num,
            };
        }

        let trimmed = line.trim();

        // Empty line
        if trimmed.is_empty() {
            if self.in_body {
                return TokenSpan {
                    token: Token::Body(String::new()),
                    line: line_num,
                };
            }
            return TokenSpan {
                token: Token::EmptyLine,
                line: line_num,
            };
        }

        // Request separator
        if trimmed.starts_with("###") {
            self.in_body = false;
            // Title (if any) becomes a comment; separator itself has no text.
            return TokenSpan {
                token: Token::RequestSeparator,
                line: line_num,
            };
        }

        // Annotations and comments - body is always over when we see a # line
        if trimmed.starts_with('#') {
            self.in_body = false;
            let comment = trimmed.trim_start_matches('#').trim();

            // Check for annotations
            if comment.starts_with('@') {
                return TokenSpan {
                    token: self.parse_annotation(comment),
                    line: line_num,
                };
            }

            return TokenSpan {
                token: Token::Comment(comment.to_string()),
                line: line_num,
            };
        }

        // If we're in body mode, everything is body content
        if self.in_body {
            return TokenSpan {
                token: Token::Body(line.to_string()),
                line: line_num,
            };
        }

        // Check for HTTP method at start of line
        let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
        if let Some(method) = parts.first() {
            let method_upper = method.to_uppercase();
            if is_http_method(&method_upper) {
                // If there's a URL on the same line, store it as pending token
                if let Some(url) = parts.get(1).map(|s| s.trim())
                    && !url.is_empty()
                {
                    self.pending_token = Some(TokenSpan {
                        token: Token::Url(url.to_string()),
                        line: line_num,
                    });
                }
                return TokenSpan {
                    token: Token::Method(method_upper),
                    line: line_num,
                };
            }
        }

        // A line beginning with a URL scheme is a URL, never a header. Decided
        // *before* the header rule because `find(':')` would otherwise split
        // `https://api.test/x` into key `https` / value `//api.test/x` - and
        // inside a header block that is silent, putting a header named after the
        // scheme on the wire. No real header name is a scheme followed by `//`,
        // so claiming these lines here is safe. The `{{var}}` URL form below
        // must stay *after* the header rule, or `Authorization: Bearer {{token}}`
        // would lex as a URL.
        if URL_SCHEMES.iter().any(|scheme| trimmed.starts_with(scheme)) {
            return TokenSpan {
                token: Token::Url(trimmed.to_string()),
                line: line_num,
            };
        }

        // Check for header (Key: Value)
        // Headers must not start with { or [ (which would be JSON body)
        if !trimmed.starts_with('{')
            && !trimmed.starts_with('[')
            && let Some(colon_pos) = trimmed.find(':')
        {
            let key = trimmed[..colon_pos].trim();
            let value = trimmed[colon_pos + 1..].trim();

            // Headers can't have spaces in the key and must be alphanumeric with dashes
            if !key.contains(' ')
                && !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                return TokenSpan {
                    token: Token::Header(key.to_string(), value.to_string()),
                    line: line_num,
                };
            }
        }

        // If we have a URL-like pattern ({{var}} substitution only matches when not inside JSON)
        if trimmed.contains("{{") && !trimmed.starts_with('{') && !trimmed.starts_with('[') {
            return TokenSpan {
                token: Token::Url(trimmed.to_string()),
                line: line_num,
            };
        }

        // Otherwise it's body content
        self.in_body = true;
        TokenSpan {
            token: Token::Body(line.to_string()),
            line: line_num,
        }
    }

    fn parse_annotation(&mut self, comment: &str) -> Token {
        let annotation = comment.trim_start_matches('@');

        // Script markers
        if annotation == "pre-script" || annotation.starts_with("pre-script ") {
            self.in_script = Some(ScriptType::PreScript);
            return Token::ScriptMarker(ScriptType::PreScript);
        }
        if annotation == "post-script" || annotation.starts_with("post-script ") {
            self.in_script = Some(ScriptType::PostScript);
            return Token::ScriptMarker(ScriptType::PostScript);
        }
        if annotation == "tests" || annotation.starts_with("tests ") {
            self.in_script = Some(ScriptType::Tests);
            return Token::ScriptMarker(ScriptType::Tests);
        }

        // Variable extraction: @set name = expression
        if annotation.starts_with("set ") {
            let rest = annotation.trim_start_matches("set ").trim();
            if let Some(eq_pos) = rest.find('=') {
                let name = rest[..eq_pos].trim().to_string();
                let expr = rest[eq_pos + 1..].trim().to_string();
                return Token::SetVariable(name, expr);
            }
        }

        // Regular annotation: @key value or @key
        let parts: Vec<&str> = annotation.splitn(2, char::is_whitespace).collect();
        let key = parts[0].to_string();
        let value = parts.get(1).map(|s| s.trim().to_string());

        Token::Annotation(key, value)
    }
}

fn is_http_method(s: &str) -> bool {
    matches!(
        s,
        "GET"
            | "POST"
            | "PUT"
            | "PATCH"
            | "DELETE"
            | "HEAD"
            | "OPTIONS"
            | "CONNECT"
            | "TRACE"
            | "WEBSOCKET"
            | "WS"
            | "GRPC"
    )
}

impl<'a> Iterator for Lexer<'a> {
    type Item = TokenSpan;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token();
        if token.token == Token::Eof {
            None
        } else {
            Some(token)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_simple_get() {
        let content = "GET https://api.example.com/users";
        let mut lexer = Lexer::new(content);

        assert!(matches!(lexer.next_token().token, Token::Method(m) if m == "GET"));
    }

    #[test]
    fn test_lexer_with_headers() {
        let content = r#"GET https://api.example.com/users
Authorization: Bearer token123
Content-Type: application/json"#;

        let mut lexer = Lexer::new(content);
        assert!(matches!(lexer.next_token().token, Token::Method(_)));
        assert!(matches!(lexer.next_token().token, Token::Url(_)));
        assert!(matches!(lexer.next_token().token, Token::Header(k, _) if k == "Authorization"));
        assert!(matches!(lexer.next_token().token, Token::Header(k, _) if k == "Content-Type"));
    }

    #[test]
    fn test_lexer_annotations() {
        let content = r#"### My Request
# @name my-request
# @protocol graphql
GET https://api.example.com"#;

        let mut lexer = Lexer::new(content);
        assert!(matches!(lexer.next_token().token, Token::RequestSeparator));
        assert!(
            matches!(lexer.next_token().token, Token::Annotation(k, Some(v)) if k == "name" && v == "my-request")
        );
        assert!(
            matches!(lexer.next_token().token, Token::Annotation(k, Some(v)) if k == "protocol" && v == "graphql")
        );
    }

    #[test]
    fn test_lexer_token_line_numbers() {
        // Each token should carry the 1-indexed line it was actually read from,
        // regardless of how far the lexer's internal cursor has advanced.
        let content = "### My Request\n# @name my-request\n\nGET https://api.example.com\nAuthorization: Bearer tok";
        let mut lexer = Lexer::new(content);

        let sep = lexer.next_token();
        assert_eq!(sep.line, 1);
        let name = lexer.next_token();
        assert_eq!(name.line, 2);
        let empty = lexer.next_token();
        assert_eq!(empty.line, 3);
        let method = lexer.next_token();
        assert_eq!(method.line, 4);
        assert!(matches!(method.token, Token::Method(_)));
        let url = lexer.next_token();
        assert_eq!(
            url.line, 4,
            "URL on same line as method must share its line number"
        );
        assert!(matches!(url.token, Token::Url(_)));
        let header = lexer.next_token();
        assert_eq!(header.line, 5);
    }
}

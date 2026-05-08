//! Lexer for .http files
//!
//! Tokenizes .http file content into a stream of tokens for the parser.


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

/// Lexer for .http files
pub struct Lexer<'a> {
    content: &'a str,
    lines: Vec<&'a str>,
    current_line: usize,
    in_script: Option<ScriptType>,
    in_body: bool,
    /// Pending token to return on next call (used when method and URL are on same line)
    pending_token: Option<Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a str) -> Self {
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

    /// Get current line number (1-indexed)
    pub fn line_number(&self) -> usize {
        self.current_line + 1
    }

    /// Peek at the next token without consuming it
    pub fn peek(&self) -> Token {
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

    /// Get the next token
    pub fn next_token(&mut self) -> Token {
        // Check for pending token first
        if let Some(token) = self.pending_token.take() {
            return token;
        }

        if self.current_line >= self.lines.len() {
            return Token::Eof;
        }

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
            return Token::ScriptLine(line.to_string());
        }

        let trimmed = line.trim();

        // Empty line
        if trimmed.is_empty() {
            if self.in_body {
                return Token::Body(String::new());
            }
            return Token::EmptyLine;
        }

        // Request separator
        if trimmed.starts_with("###") {
            self.in_body = false;
            let title = trimmed.trim_start_matches('#').trim();
            if title.is_empty() {
                return Token::RequestSeparator;
            }
            // Return separator, the title becomes a comment
            return Token::RequestSeparator;
        }

        // Annotations and comments — body is always over when we see a # line
        if trimmed.starts_with('#') {
            self.in_body = false;
            let comment = trimmed.trim_start_matches('#').trim();

            // Check for annotations
            if comment.starts_with('@') {
                return self.parse_annotation(comment);
            }

            return Token::Comment(comment.to_string());
        }

        // If we're in body mode, everything is body content
        if self.in_body {
            return Token::Body(line.to_string());
        }

        // Check for HTTP method at start of line
        let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
        if let Some(method) = parts.first() {
            let method_upper = method.to_uppercase();
            if is_http_method(&method_upper) {
                // If there's a URL on the same line, store it as pending token
                if let Some(url) = parts.get(1).map(|s| s.trim()) {
                    if !url.is_empty() {
                        self.pending_token = Some(Token::Url(url.to_string()));
                    }
                }
                return Token::Method(method_upper);
            }
        }

        // Check for header (Key: Value)
        // Headers must not start with { or [ (which would be JSON body)
        if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
            if let Some(colon_pos) = trimmed.find(':') {
                let key = trimmed[..colon_pos].trim();
                let value = trimmed[colon_pos + 1..].trim();

                // Headers can't have spaces in the key and must be alphanumeric with dashes
                if !key.contains(' ') && !key.is_empty() && key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                    return Token::Header(key.to_string(), value.to_string());
                }
            }
        }

        // If we have a URL-like pattern ({{var}} substitution only matches when not inside JSON)
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("ws://")
            || trimmed.starts_with("wss://")
            || trimmed.starts_with("grpc://")
            || (trimmed.contains("{{") && !trimmed.starts_with('{') && !trimmed.starts_with('['))
        {
            return Token::Url(trimmed.to_string());
        }

        // Otherwise it's body content
        self.in_body = true;
        Token::Body(line.to_string())
    }

    fn parse_annotation(&mut self, comment: &str) -> Token {
        let annotation = comment.trim_start_matches('@');

        // Script markers
        if annotation == "pre-script" || annotation.starts_with("pre-script") {
            self.in_script = Some(ScriptType::PreScript);
            return Token::ScriptMarker(ScriptType::PreScript);
        }
        if annotation == "post-script" || annotation.starts_with("post-script") {
            self.in_script = Some(ScriptType::PostScript);
            return Token::ScriptMarker(ScriptType::PostScript);
        }
        if annotation == "tests" || annotation.starts_with("tests") {
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
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE"
        | "HEAD" | "OPTIONS" | "CONNECT" | "TRACE"
        | "WEBSOCKET" | "WS" | "GRPC"
    )
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token();
        if token == Token::Eof {
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

        assert!(matches!(lexer.next_token(), Token::Method(m) if m == "GET"));
    }

    #[test]
    fn test_lexer_with_headers() {
        let content = r#"GET https://api.example.com/users
Authorization: Bearer token123
Content-Type: application/json"#;

        let mut lexer = Lexer::new(content);
        assert!(matches!(lexer.next_token(), Token::Method(_)));
        assert!(matches!(lexer.next_token(), Token::Url(_)));
        assert!(matches!(lexer.next_token(), Token::Header(k, _) if k == "Authorization"));
        assert!(matches!(lexer.next_token(), Token::Header(k, _) if k == "Content-Type"));
    }

    #[test]
    fn test_lexer_annotations() {
        let content = r#"### My Request
# @name my-request
# @protocol graphql
GET https://api.example.com"#;

        let mut lexer = Lexer::new(content);
        assert!(matches!(lexer.next_token(), Token::RequestSeparator));
        assert!(matches!(lexer.next_token(), Token::Annotation(k, Some(v)) if k == "name" && v == "my-request"));
        assert!(matches!(lexer.next_token(), Token::Annotation(k, Some(v)) if k == "protocol" && v == "graphql"));
    }
}

//! Syntax highlighting for code editor

use gpui::Hsla;
use crate::theme::Colors;

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum TokenKind {
    Key,
    String,
    Number,
    Boolean,
    Null,
    Punctuation,
    Tag,
    Attribute,
    Keyword,
    Comment,
    Property,
    Plain,
}

impl TokenKind {
    pub fn color(&self, theme: &Colors) -> Hsla {
        match self {
            TokenKind::Key => theme.accent,
            TokenKind::String => theme.status_success,
            TokenKind::Number => theme.method_patch,
            TokenKind::Boolean | TokenKind::Keyword => theme.method_delete,
            TokenKind::Null => theme.method_put,
            TokenKind::Punctuation => theme.text_secondary,
            TokenKind::Tag => theme.accent,
            TokenKind::Attribute => theme.accent,
            TokenKind::Comment => theme.text_secondary.opacity(0.6),
            TokenKind::Property => theme.method_get,
            TokenKind::Plain => theme.text_primary,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub text: String,
    pub kind: TokenKind,
}

pub trait Highlighter: Send + Sync {
    fn tokenize_line(&self, line: &str) -> Vec<Token>;
}

pub struct JsonHighlighter;
pub struct XmlHighlighter;
pub struct GraphQLHighlighter;
pub struct PlainHighlighter;

impl Highlighter for JsonHighlighter {
    fn tokenize_line(&self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = line.chars().peekable();
        let mut current = String::new();
        let mut in_string = false;
        let mut is_key = false;
        let mut after_colon = false;

        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    if in_string {
                        current.push(c);
                        let kind = if is_key { TokenKind::Key } else { TokenKind::String };
                        tokens.push(Token { text: current.clone(), kind });
                        current.clear();
                        in_string = false;
                        is_key = false;
                    } else {
                        if !current.is_empty() {
                            tokens.push(Token { text: current.clone(), kind: TokenKind::Punctuation });
                            current.clear();
                        }
                        current.push(c);
                        in_string = true;
                        is_key = !after_colon;
                    }
                }
                ':' if !in_string => {
                    if !current.is_empty() {
                        tokens.push(Token { text: current.clone(), kind: TokenKind::Punctuation });
                        current.clear();
                    }
                    tokens.push(Token { text: ":".to_string(), kind: TokenKind::Punctuation });
                    after_colon = true;
                }
                ',' if !in_string => {
                    if !current.is_empty() {
                        let kind = classify_value(&current);
                        tokens.push(Token { text: current.clone(), kind });
                        current.clear();
                    }
                    tokens.push(Token { text: ",".to_string(), kind: TokenKind::Punctuation });
                    after_colon = false;
                }
                '{' | '}' | '[' | ']' if !in_string => {
                    if !current.is_empty() {
                        let kind = classify_value(&current);
                        tokens.push(Token { text: current.clone(), kind });
                        current.clear();
                    }
                    tokens.push(Token { text: c.to_string(), kind: TokenKind::Punctuation });
                    if c == '{' || c == '[' {
                        after_colon = false;
                    }
                }
                '\\' if in_string => {
                    current.push(c);
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            let kind = if in_string {
                if is_key { TokenKind::Key } else { TokenKind::String }
            } else {
                classify_value(&current)
            };
            tokens.push(Token { text: current, kind });
        }

        tokens
    }
}

fn classify_value(s: &str) -> TokenKind {
    let trimmed = s.trim();
    if trimmed.parse::<f64>().is_ok() {
        TokenKind::Number
    } else if trimmed == "true" || trimmed == "false" {
        TokenKind::Boolean
    } else if trimmed == "null" {
        TokenKind::Null
    } else {
        TokenKind::Punctuation
    }
}

impl Highlighter for XmlHighlighter {
    fn tokenize_line(&self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = line.chars().peekable();
        let mut current = String::new();
        let mut in_tag = false;
        let mut in_attr_value = false;

        while let Some(c) = chars.next() {
            match c {
                '<' => {
                    if !current.is_empty() {
                        tokens.push(Token { text: current.clone(), kind: TokenKind::Plain });
                        current.clear();
                    }
                    current.push(c);
                    in_tag = true;
                }
                '>' if in_tag => {
                    current.push(c);
                    tokens.push(Token { text: current.clone(), kind: TokenKind::Tag });
                    current.clear();
                    in_tag = false;
                }
                '"' if in_tag => {
                    if in_attr_value {
                        current.push(c);
                        tokens.push(Token { text: current.clone(), kind: TokenKind::String });
                        current.clear();
                        in_attr_value = false;
                    } else {
                        if !current.is_empty() {
                            tokens.push(Token { text: current.clone(), kind: TokenKind::Tag });
                            current.clear();
                        }
                        current.push(c);
                        in_attr_value = true;
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            let kind = if in_tag { TokenKind::Tag } else { TokenKind::Plain };
            tokens.push(Token { text: current, kind });
        }

        tokens
    }
}

impl Highlighter for GraphQLHighlighter {
    fn tokenize_line(&self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = line.chars().peekable();
        let mut current = String::new();

        // GraphQL keywords
        let keywords = [
            "query", "mutation", "subscription", "fragment", "on", "type",
            "input", "enum", "scalar", "interface", "union", "implements",
            "extend", "schema", "directive", "repeatable",
        ];
        let builtins = ["String", "Int", "Float", "Boolean", "ID", "true", "false", "null"];

        let flush_word = |current: &mut String, tokens: &mut Vec<Token>| {
            if current.is_empty() { return; }
            let kind = if keywords.contains(&current.as_str()) {
                TokenKind::Keyword
            } else if builtins.contains(&current.as_str()) {
                TokenKind::Boolean
            } else if current.starts_with('$') {
                TokenKind::Property
            } else if current.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                TokenKind::Tag // Type names
            } else {
                TokenKind::Key // Field names
            };
            tokens.push(Token { text: current.clone(), kind });
            current.clear();
        };

        while let Some(c) = chars.next() {
            match c {
                '#' => {
                    flush_word(&mut current, &mut tokens);
                    let mut comment = c.to_string();
                    comment.extend(chars.by_ref());
                    tokens.push(Token { text: comment, kind: TokenKind::Comment });
                    break;
                }
                '"' => {
                    flush_word(&mut current, &mut tokens);
                    let mut s = c.to_string();
                    // Handle triple quotes
                    if chars.peek() == Some(&'"') {
                        s.push(chars.next().unwrap());
                        if chars.peek() == Some(&'"') {
                            s.push(chars.next().unwrap());
                            // Read until closing """
                            while let Some(ch) = chars.next() {
                                s.push(ch);
                                if ch == '"' && chars.peek() == Some(&'"') {
                                    s.push(chars.next().unwrap());
                                    if chars.peek() == Some(&'"') {
                                        s.push(chars.next().unwrap());
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        while let Some(ch) = chars.next() {
                            s.push(ch);
                            if ch == '"' { break; }
                            if ch == '\\' { if let Some(esc) = chars.next() { s.push(esc); } }
                        }
                    }
                    tokens.push(Token { text: s, kind: TokenKind::String });
                }
                '{' | '}' | '(' | ')' | '[' | ']' | ':' | '!' | '@' | '=' | '|' | '&' | ',' => {
                    flush_word(&mut current, &mut tokens);
                    tokens.push(Token { text: c.to_string(), kind: TokenKind::Punctuation });
                }
                '$' => {
                    flush_word(&mut current, &mut tokens);
                    current.push(c);
                }
                _ if c.is_whitespace() => {
                    flush_word(&mut current, &mut tokens);
                    tokens.push(Token { text: c.to_string(), kind: TokenKind::Plain });
                }
                _ if c.is_alphanumeric() || c == '_' => {
                    current.push(c);
                }
                _ => {
                    flush_word(&mut current, &mut tokens);
                    tokens.push(Token { text: c.to_string(), kind: TokenKind::Plain });
                }
            }
        }
        flush_word(&mut current, &mut tokens);
        tokens
    }
}

impl Highlighter for PlainHighlighter {
    fn tokenize_line(&self, line: &str) -> Vec<Token> {
        vec![Token { text: line.to_string(), kind: TokenKind::Plain }]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Language {
    Json,
    Xml,
    Html,
    JavaScript,
    GraphQL,
    Shell,
    Python,
    Go,
    Rust,
    Plain,
}

impl Language {
    pub fn highlighter(&self) -> Box<dyn Highlighter> {
        match self {
            Language::Json => Box::new(JsonHighlighter),
            Language::Xml | Language::Html => Box::new(XmlHighlighter),
            Language::JavaScript => Box::new(JavaScriptHighlighter),
            Language::GraphQL => Box::new(GraphQLHighlighter),
            Language::Shell => Box::new(ShellHighlighter),
            Language::Python => Box::new(PythonHighlighter),
            Language::Go => Box::new(GoHighlighter),
            Language::Rust => Box::new(RustHighlighter),
            Language::Plain => Box::new(PlainHighlighter),
        }
    }

    #[allow(dead_code)]
    pub fn detect(content: &str) -> Self {
        let trimmed = content.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            Language::Json
        } else if trimmed.starts_with('<') {
            if trimmed.contains("<!DOCTYPE html") || trimmed.contains("<html") {
                Language::Html
            } else {
                Language::Xml
            }
        } else {
            Language::Plain
        }
    }
}

pub struct ShellHighlighter;
pub struct PythonHighlighter;
pub struct GoHighlighter;
pub struct RustHighlighter;

fn tokenize_generic(line: &str, is_keyword: impl Fn(&str) -> bool, comment_prefix: &str, double_slash_comment: bool) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        if c.is_whitespace() {
            tokens.push(Token { text: c.to_string(), kind: TokenKind::Plain });
        } else if double_slash_comment && c == '/' && chars.peek().map(|(_, c)| *c) == Some('/') {
            tokens.push(Token { text: line[i..].to_string(), kind: TokenKind::Comment });
            break;
        } else if !double_slash_comment && c == comment_prefix.chars().next().unwrap_or('\0') && comment_prefix.len() == 1 {
            tokens.push(Token { text: line[i..].to_string(), kind: TokenKind::Comment });
            break;
        } else if c == '"' || c == '\'' {
            let quote = c;
            let start = i;
            let mut end = i + 1;
            while let Some((j, ch)) = chars.next() {
                end = j + ch.len_utf8();
                if ch == quote { break; }
                if ch == '\\' { chars.next(); }
            }
            tokens.push(Token { text: line[start..end].to_string(), kind: TokenKind::String });
        } else if c.is_ascii_digit() {
            let start = i;
            let mut end = i + 1;
            while let Some(&(j, ch)) = chars.peek() {
                if ch.is_ascii_digit() || ch == '.' {
                    end = j + ch.len_utf8();
                    chars.next();
                } else { break; }
            }
            tokens.push(Token { text: line[start..end].to_string(), kind: TokenKind::Number });
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            let mut end = i + c.len_utf8();
            while let Some(&(j, ch)) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    end = j + ch.len_utf8();
                    chars.next();
                } else { break; }
            }
            let word = &line[start..end];
            let kind = if is_keyword(word) { TokenKind::Keyword } else { TokenKind::Plain };
            tokens.push(Token { text: word.to_string(), kind });
        } else if "(){}[].,;:=<>&|!".contains(c) {
            tokens.push(Token { text: c.to_string(), kind: TokenKind::Punctuation });
        } else {
            tokens.push(Token { text: c.to_string(), kind: TokenKind::Plain });
        }
    }
    tokens
}

impl Highlighter for ShellHighlighter {
    fn tokenize_line(&self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = line.char_indices().peekable();

        while let Some((i, c)) = chars.next() {
            if c == '#' {
                tokens.push(Token { text: line[i..].to_string(), kind: TokenKind::Comment });
                break;
            } else if c == '\'' {
                let start = i;
                let mut end = i + 1;
                while let Some((j, ch)) = chars.next() {
                    end = j + ch.len_utf8();
                    if ch == '\'' { break; }
                }
                tokens.push(Token { text: line[start..end].to_string(), kind: TokenKind::String });
            } else if c == '"' {
                let start = i;
                let mut end = i + 1;
                while let Some((j, ch)) = chars.next() {
                    end = j + ch.len_utf8();
                    if ch == '"' { break; }
                    if ch == '\\' { chars.next(); }
                }
                tokens.push(Token { text: line[start..end].to_string(), kind: TokenKind::String });
            } else if c == '-' {
                // flags like -H, --data-raw
                let start = i;
                let mut end = i + 1;
                while let Some(&(j, ch)) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                        end = j + ch.len_utf8();
                        chars.next();
                    } else { break; }
                }
                tokens.push(Token { text: line[start..end].to_string(), kind: TokenKind::Keyword });
            } else if c.is_alphabetic() || c == '_' {
                let start = i;
                let mut end = i + c.len_utf8();
                while let Some(&(j, ch)) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        end = j + ch.len_utf8();
                        chars.next();
                    } else { break; }
                }
                let word = &line[start..end];
                let kind = match word {
                    "curl" | "http" | "https" => TokenKind::Keyword,
                    _ => TokenKind::Plain,
                };
                tokens.push(Token { text: word.to_string(), kind });
            } else if c.is_whitespace() {
                tokens.push(Token { text: c.to_string(), kind: TokenKind::Plain });
            } else {
                tokens.push(Token { text: c.to_string(), kind: TokenKind::Plain });
            }
        }
        tokens
    }
}

impl Highlighter for PythonHighlighter {
    fn tokenize_line(&self, line: &str) -> Vec<Token> {
        tokenize_generic(line, |w| matches!(w,
            "import" | "from" | "def" | "class" | "return" | "if" | "elif" | "else" |
            "for" | "while" | "with" | "as" | "in" | "not" | "and" | "or" | "is" |
            "try" | "except" | "finally" | "raise" | "pass" | "break" | "continue" |
            "True" | "False" | "None" | "lambda" | "yield" | "async" | "await" |
            "print" | "len" | "range" | "open" | "type"
        ), "#", false)
    }
}

impl Highlighter for GoHighlighter {
    fn tokenize_line(&self, line: &str) -> Vec<Token> {
        tokenize_generic(line, |w| matches!(w,
            "package" | "import" | "func" | "var" | "const" | "type" | "struct" | "interface" |
            "if" | "else" | "for" | "range" | "return" | "switch" | "case" | "default" |
            "break" | "continue" | "goto" | "defer" | "go" | "select" | "chan" | "map" |
            "nil" | "true" | "false" | "error" | "string" | "int" | "bool" | "byte" |
            "make" | "new" | "append" | "len" | "cap" | "close" | "panic" | "recover"
        ), "", true)
    }
}

impl Highlighter for RustHighlighter {
    fn tokenize_line(&self, line: &str) -> Vec<Token> {
        tokenize_generic(line, |w| matches!(w,
            "use" | "fn" | "let" | "mut" | "pub" | "mod" | "impl" | "struct" | "enum" |
            "trait" | "type" | "const" | "static" | "if" | "else" | "for" | "while" |
            "loop" | "match" | "return" | "break" | "continue" | "move" | "ref" | "in" |
            "where" | "self" | "Self" | "super" | "crate" | "async" | "await" | "dyn" |
            "true" | "false" | "Some" | "None" | "Ok" | "Err" | "Box" | "Vec" | "String"
        ), "", true)
    }
}

/// JavaScript syntax highlighter
pub struct JavaScriptHighlighter;

impl Highlighter for JavaScriptHighlighter {
    fn tokenize_line(&self, line: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut chars = line.char_indices().peekable();

        while let Some((i, c)) = chars.next() {
            if c.is_whitespace() {
                tokens.push(Token { text: c.to_string(), kind: TokenKind::Plain });
            } else if c == '/' && chars.peek().map(|(_, c)| *c) == Some('/') {
                // Line comment
                tokens.push(Token { text: line[i..].to_string(), kind: TokenKind::Comment });
                break;
            } else if c == '"' || c == '\'' || c == '`' {
                // String
                let quote = c;
                let start = i;
                let mut end = i + 1;
                while let Some((j, ch)) = chars.next() {
                    end = j + ch.len_utf8();
                    if ch == quote {
                        break;
                    }
                    if ch == '\\' {
                        if let Some((_, _)) = chars.next() {
                            continue;
                        }
                    }
                }
                tokens.push(Token { text: line[start..end].to_string(), kind: TokenKind::String });
            } else if c.is_ascii_digit() {
                // Number
                let start = i;
                let mut end = i + 1;
                while let Some(&(j, ch)) = chars.peek() {
                    if ch.is_ascii_digit() || ch == '.' || ch == 'e' || ch == 'E' {
                        end = j + ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token { text: line[start..end].to_string(), kind: TokenKind::Number });
            } else if c.is_alphabetic() || c == '_' || c == '$' {
                // Identifier or keyword
                let start = i;
                let mut end = i + c.len_utf8();
                while let Some(&(j, ch)) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                        end = j + ch.len_utf8();
                        chars.next();
                    } else {
                        break;
                    }
                }
                let word = &line[start..end];
                let kind = match word {
                    "const" | "let" | "var" | "function" | "return" | "if" | "else" |
                    "for" | "while" | "do" | "switch" | "case" | "break" | "continue" |
                    "new" | "this" | "class" | "extends" | "import" | "export" |
                    "default" | "try" | "catch" | "finally" | "throw" | "async" | "await" |
                    "typeof" | "instanceof" | "in" | "of" | "delete" | "void" | "yield" => TokenKind::Keyword,
                    "true" | "false" | "null" | "undefined" | "NaN" | "Infinity" => TokenKind::Keyword,
                    "console" | "request" | "response" | "env" | "expect" => TokenKind::Property,
                    _ => TokenKind::Plain,
                };
                tokens.push(Token { text: word.to_string(), kind });
            } else if "(){}[].,;:?!+-*/%=<>&|^~".contains(c) {
                tokens.push(Token { text: c.to_string(), kind: TokenKind::Punctuation });
            } else {
                tokens.push(Token { text: c.to_string(), kind: TokenKind::Plain });
            }
        }

        tokens
    }
}

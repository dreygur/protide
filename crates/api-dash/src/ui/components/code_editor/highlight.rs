//! Syntax highlighting for code editor

use gpui::Hsla;
use crate::theme::Colors;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenKind {
    Key,
    String,
    Number,
    Boolean,
    Null,
    Punctuation,
    Tag,
    Attribute,
    Plain,
}

impl TokenKind {
    pub fn color(&self, theme: &Colors) -> Hsla {
        match self {
            TokenKind::Key => theme.accent,
            TokenKind::String => theme.status_success,
            TokenKind::Number => theme.method_patch,
            TokenKind::Boolean => theme.method_delete,
            TokenKind::Null => theme.method_put,
            TokenKind::Punctuation => theme.text_muted,
            TokenKind::Tag => theme.accent,
            TokenKind::Attribute => theme.accent,
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
    Plain,
}

impl Language {
    pub fn highlighter(&self) -> Box<dyn Highlighter> {
        match self {
            Language::Json => Box::new(JsonHighlighter),
            Language::Xml | Language::Html => Box::new(XmlHighlighter),
            Language::Plain => Box::new(PlainHighlighter),
        }
    }

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

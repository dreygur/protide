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

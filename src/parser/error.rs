//! Error types for the HTTP parser.

use thiserror::Error;

/// Errors that can occur during HTTP request parsing.
#[derive(Debug, Error)]
pub enum Error {
    /// The HTTP method in the request is not supported.
    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(String),

    /// The request path is invalid or missing.
    #[error("Invalid HTTP path")]
    InvalidPath,

    /// The request line is malformed (wrong format or missing components).
    #[error("Malformed request line: {0}")]
    MalformedRequestLine(String),

    /// The HTTP version in the request is not supported.
    #[error("Invalid HTTP version: {0}")]
    InvalidVersion(String),

    /// A required header is missing from the request.
    #[error("Required header is missing: {0}")]
    MissingHeader(String),

    /// A header in the request has an invalid format.
    #[error("Invalid header format")]
    InvalidHeaderFormat,

    /// The request is empty.
    #[error("Empty request")]
    EmptyRequest,

    /// Error parsing JSON.
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}
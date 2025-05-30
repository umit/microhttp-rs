//! Error types for the HTTP server.

use thiserror::Error;

use crate::parser::{Error as ParserError, Method};

/// Errors that can occur during HTTP server operation.
#[derive(Debug, Error)]
pub enum Error {
    /// Error parsing an HTTP request.
    #[error("Parse error: {0}")]
    ParseError(#[from] ParserError),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Requested resource not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Method not allowed for the requested resource.
    #[error("Method {0} not allowed for path: {1}")]
    MethodNotAllowed(Method, String),

    /// Internal server error.
    #[error("Internal server error: {0}")]
    InternalError(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
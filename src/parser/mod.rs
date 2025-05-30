//! HTTP parser module.
//!
//! This module provides functionality for parsing HTTP requests with a focus on simplicity,
//! correctness, and performance.

mod request;
mod method;
mod version;
mod error;

// Re-export public items
pub use request::HttpRequest;
pub use method::Method;
pub use version::HttpVersion;
pub use error::Error;

// Re-export the parse_request function
pub use request::parse_request;

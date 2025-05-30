//! HTTP server implementation for microhttp-rs.
//!
//! This module provides a simple, efficient HTTP server implementation
//! that leverages Rust's concurrency features and the microhttp-rs parser.

mod response;
mod config;
mod error;
mod handler;
mod http_server;
mod tests;

// Re-export public items
pub use response::{HttpResponse, StatusCode};
pub use config::ServerConfig;
pub use error::Error;
pub use http_server::HttpServer;

//! A minimal HTTP parser library.
//!
//! This library provides functionality for parsing HTTP requests with a focus on simplicity,
//! correctness, and performance.
//!
//! # Features
//!
//! - Parse HTTP requests from byte slices
//! - Support for common HTTP methods (GET, POST, PUT, DELETE, HEAD, OPTIONS, PATCH)
//! - Support for HTTP versions 1.0, 1.1, and 2.0
//! - Proper error handling with descriptive error messages
//! - No external dependencies for the core parsing functionality
//!
//! # Examples
//!
//! ## Basic usage
//!
//! ```
//! use microhttp_rs::parse_request;
//!
//! let request_bytes = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
//!
//! match parse_request(request_bytes) {
//!     Ok(request) => {
//!         println!("Method: {}", request.method);
//!         println!("Path: {}", request.path);
//!         println!("Version: {}", request.version);
//!         println!("Headers: {:?}", request.headers);
//!     },
//!     Err(err) => {
//!         println!("Error parsing request: {}", err);
//!     }
//! }
//! ```
//!
//! ## Error handling
//!
//! ```
//! use microhttp_rs::{parse_request, ParserError};
//!
//! let invalid_request = b"INVALID /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
//!
//! match parse_request(invalid_request) {
//!     Ok(_) => println!("Request parsed successfully"),
//!     Err(ParserError::InvalidMethod(method)) => println!("Invalid method: {}", method),
//!     Err(ParserError::MalformedRequestLine(line)) => println!("Malformed request line: {}", line),
//!     Err(err) => println!("Other error: {}", err),
//! }
//! ```
//!
//! See the `examples` directory for more complete examples, including a simple HTTP server.

// Export the parser module
pub mod parser;

// Export the server module
pub mod server;

// Re-export commonly used items for convenience
pub use parser::{Error as ParserError, HttpRequest, HttpVersion, Method, parse_request};
pub use server::{Error as ServerError, HttpResponse, HttpServer, ServerConfig, StatusCode};

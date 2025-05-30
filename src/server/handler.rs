//! HTTP request handlers and routing.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::parser::{HttpRequest, Method};
use crate::server::{HttpResponse, Error};

/// Type alias for a boxed future that returns a Result<HttpResponse, Error>.
pub type HandlerFuture = Pin<Box<dyn Future<Output = Result<HttpResponse, Error>> + Send>>;

/// Type alias for a handler function that takes an HttpRequest and returns a HandlerFuture.
pub type HandlerFn = Arc<dyn Fn(HttpRequest) -> HandlerFuture + Send + Sync>;

/// Represents a route in the HTTP server.
pub struct Route {
    /// The path to match.
    pub path: String,
    /// The HTTP methods to match.
    pub methods: Vec<Method>,
    /// The handler function.
    pub handler: HandlerFn,
}
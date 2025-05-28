//! HTTP server implementation for microhttp-rs.
//!
//! This module provides a simple, efficient HTTP server implementation
//! that leverages Rust's concurrency features and the microhttp-rs parser.
use serde::Serialize;
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinSet;
use tokio::signal;
use log::{info, warn, error};

use crate::parser::{Error as ParserError, HttpRequest, Method};

/// HTTP status codes with their standard reason phrases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
}

impl StatusCode {
    /// Get the reason phrase for this status code.
    pub fn reason_phrase(&self) -> &'static str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::Created => "Created",
            StatusCode::Accepted => "Accepted",
            StatusCode::NoContent => "No Content",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::Unauthorized => "Unauthorized",
            StatusCode::Forbidden => "Forbidden",
            StatusCode::NotFound => "Not Found",
            StatusCode::MethodNotAllowed => "Method Not Allowed",
            StatusCode::InternalServerError => "Internal Server Error",
            StatusCode::NotImplemented => "Not Implemented",
            StatusCode::BadGateway => "Bad Gateway",
            StatusCode::ServiceUnavailable => "Service Unavailable",
        }
    }
}

/// Represents an HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// The HTTP status code
    pub status: StatusCode,
    /// The HTTP headers
    pub headers: HashMap<String, String>,
    /// The response body
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Create a new HTTP response with the given status code.
    pub fn new(status: StatusCode) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Server".to_string(), "microhttp-rs".to_string());

        Self {
            status,
            headers,
            body: Vec::new(),
        }
    }

    /// Set the response body with a string.
    pub fn with_body_string(mut self, body: impl Into<String>) -> Self {
        let body_string = body.into();
        self.body = body_string.into_bytes();
        let content_length = self.body.len().to_string();
        self.with_header("Content-Length", content_length)
    }

    /// Set the response body with bytes.
    pub fn with_body_bytes(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        let content_length = self.body.len().to_string();
        self.with_header("Content-Length", content_length)
    }

    /// Add or replace a header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Set the content type.
    pub fn with_content_type(self, content_type: impl Into<String>) -> Self {
        self.with_header("Content-Type", content_type)
    }

    /// Set the response body with a JSON value.
    ///
    /// This method serializes the provided value to JSON and sets it as the response body.
    /// It also sets the Content-Type header to "application/json".
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type to serialize to JSON. Must implement `Serialize`.
    ///
    /// # Arguments
    ///
    /// * `value` - The value to serialize to JSON
    ///
    /// # Returns
    ///
    /// * `Result<Self, Error>` - The updated response or an error
    ///
    /// # Examples
    ///
    /// ```
    /// use serde::Serialize;
    /// use microhttp_rs::{HttpResponse, StatusCode};
    ///
    /// #[derive(Serialize)]
    /// struct User {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// let user = User {
    ///     name: "John".to_string(),
    ///     age: 30,
    /// };
    ///
    /// let response = HttpResponse::new(StatusCode::Ok)
    ///     .with_json(&user)
    ///     .unwrap();
    /// ```
    pub fn with_json<T>(self, value: &T) -> Result<Self, Error>
    where
        T: Serialize,
    {
        let json = serde_json::to_vec(value)?;
        Ok(self
            .with_content_type("application/json")
            .with_body_bytes(json))
    }

    /// Convert the response to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Status line
        let status_line = format!(
            "HTTP/1.1 {} {}\r\n",
            self.status as u16,
            self.status.reason_phrase()
        );
        result.extend_from_slice(status_line.as_bytes());

        // Headers
        for (name, value) in &self.headers {
            let header_line = format!("{}: {}\r\n", name, value);
            result.extend_from_slice(header_line.as_bytes());
        }

        // Empty line separating headers from body
        result.extend_from_slice(b"\r\n");

        // Body
        result.extend_from_slice(&self.body);

        result
    }
}

/// Errors that can occur during server operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error parsing the HTTP request.
    #[error("Request parsing error: {0}")]
    ParseError(#[from] ParserError),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// No handler found for the requested path.
    #[error("No handler found for path: {0}")]
    NotFound(String),

    /// Method not allowed for the requested path.
    #[error("Method {0} not allowed for path: {1}")]
    MethodNotAllowed(Method, String),

    /// Internal server error.
    #[error("Internal server error: {0}")]
    InternalError(String),

    /// Error serializing or deserializing JSON.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Type alias for a handler function.
pub type HandlerFn = Arc<
    dyn Fn(HttpRequest) -> Pin<Box<dyn Future<Output = Result<HttpResponse, Error>> + Send>>
        + Send
        + Sync,
>;

/// A route in the HTTP server.
#[derive(Clone)]
pub struct Route {
    /// The path pattern to match.
    pub path: String,
    /// The HTTP methods this route handles.
    pub methods: Vec<Method>,
    /// The handler function.
    pub handler: HandlerFn,
}

/// HTTP server configuration.
#[derive(Clone)]
pub struct ServerConfig {
    /// The address to bind to.
    pub addr: SocketAddr,
    /// The maximum number of concurrent connections.
    pub max_connections: usize,
    /// The read buffer size.
    pub read_buffer_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1:8080".parse().unwrap(),
            max_connections: 1024,
            read_buffer_size: 8192,
        }
    }
}

/// An HTTP server.
pub struct HttpServer {
    /// The server configuration.
    pub config: ServerConfig,
    /// The routes.
    pub routes: Arc<RwLock<Vec<Route>>>,
}

impl HttpServer {
    /// Create a new HTTP server with the given configuration.
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            routes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a route to the server.
    pub async fn add_route<F, Fut>(&self, path: impl Into<String>, methods: Vec<Method>, handler: F)
    where
        F: Fn(HttpRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<HttpResponse, Error>> + Send + 'static,
    {
        let path = path.into();
        let handler = Arc::new(move |req: HttpRequest| -> Pin<Box<dyn Future<Output = Result<HttpResponse, Error>> + Send>> {
            Box::pin(handler(req))
        });

        let route = Route {
            path,
            methods,
            handler,
        };

        self.routes.write().await.push(route);
    }

    /// Display the server banner and registered endpoints.
    async fn display_server_info(&self) -> Result<(), Error> {
        // Display the banner
        let banner = include_str!("banner.txt");
        info!("\n{}", banner);

        // Display registered endpoints
        let routes = self.routes.read().await;
        info!("Registered endpoints:");
        for route in routes.iter() {
            let methods = route.methods.iter()
                .map(|m| format!("{}", m))
                .collect::<Vec<String>>()
                .join(", ");
            info!("  {} {}", methods, route.path);
        }

        Ok(())
    }

    /// Set up the TCP listener.
    async fn setup_listener(&self) -> Result<TcpListener, Error> {
        let listener = TcpListener::bind(&self.config.addr).await?;
        info!("Server listening on http://{}", self.config.addr);
        Ok(listener)
    }

    /// Set up a Ctrl+C handler for graceful shutdown.
    fn setup_ctrl_c_handler(shutdown_tx: Arc<mpsc::Sender<()>>, tasks: &mut JoinSet<()>) {
        let shutdown_tx_clone = shutdown_tx.clone();
        tasks.spawn(async move {
            match signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received shutdown signal, stopping server...");
                    let _ = shutdown_tx_clone.send(()).await;
                }
                Err(err) => {
                    error!("Error setting up Ctrl+C handler: {}", err);
                }
            }
        });
    }

    /// Handle a new connection.
    async fn handle_new_connection(
        mut socket: tokio::net::TcpStream,
        addr: SocketAddr,
        semaphore: Arc<tokio::sync::Semaphore>,
        routes: Arc<RwLock<Vec<Route>>>,
        read_buffer_size: usize,
        shutdown_tx: Arc<mpsc::Sender<()>>,
        tasks: &mut JoinSet<()>,
    ) {

        // Try to acquire a permit from the semaphore
        let permit = match semaphore.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => {
                warn!(
                    "Connection limit reached, rejecting connection from {}",
                    addr
                );
                // Send a 503 Service Unavailable response
                let response = HttpResponse::new(StatusCode::ServiceUnavailable)
                    .with_content_type("text/plain")
                    .with_body_string("Server is at capacity, please try again later");
                let _ = socket.write_all(&response.to_bytes()).await;
                return;
            }
        };

        // Clone references for the task
        let routes = routes.clone();
        let shutdown_tx = shutdown_tx.clone();

        // Spawn a task to handle the connection
        tasks.spawn(async move {
            // The permit is dropped when the task completes, releasing the semaphore slot
            let _permit = permit;

            if let Err(e) = Self::handle_connection(&mut socket, routes, read_buffer_size).await {
                error!("Error handling connection: {}", e);

                // If there's a critical error, signal shutdown
                if matches!(e, Error::IoError(_)) {
                    info!("Critical I/O error, initiating shutdown");
                    let _ = shutdown_tx.send(()).await;
                }
            }
        });
    }

    /// Handle connection errors.
    async fn handle_connection_error(e: std::io::Error) -> bool {
        error!("Error accepting connection: {}", e);

        // If there's a critical error, signal to break the loop
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            error!("Critical error accepting connection, shutting down");
            return true;
        }

        // For other errors, wait a bit before retrying
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        false
    }

    /// Perform graceful shutdown.
    async fn perform_shutdown(tasks: &mut JoinSet<()>) {
        // Wait for all tasks to complete (with timeout)
        info!("Waiting for {} active connections to complete...", tasks.len());
        let shutdown_timeout = tokio::time::Duration::from_secs(30);
        let _ = tokio::time::timeout(shutdown_timeout, async {
            while let Some(res) = tasks.join_next().await {
                if let Err(e) = res {
                    error!("Task failed during shutdown: {}", e);
                }
            }
        }).await;

        info!("Server shutdown complete");
    }

    /// Start the server and listen for incoming connections.
    pub async fn start(&self) -> Result<(), Error> {
        // Display server information
        self.display_server_info().await?;

        // Set up the TCP listener
        let listener = self.setup_listener().await?;

        // Create a semaphore to limit concurrent connections
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_connections));

        // Create a channel for shutdown signaling
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let shutdown_tx = Arc::new(shutdown_tx);

        // Use JoinSet to keep track of all spawned tasks
        let mut tasks = JoinSet::new();

        // Set up a Ctrl+C handler for graceful shutdown
        Self::setup_ctrl_c_handler(shutdown_tx.clone(), &mut tasks);

        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Shutting down server...");
                    break;
                }

                // Accept new connections
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((socket, addr)) => {
                            Self::handle_new_connection(
                                socket, 
                                addr, 
                                semaphore.clone(), 
                                self.routes.clone(), 
                                self.config.read_buffer_size, 
                                shutdown_tx.clone(), 
                                &mut tasks
                            ).await;
                        },
                        Err(e) => {
                            if Self::handle_connection_error(e).await {
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Perform graceful shutdown
        Self::perform_shutdown(&mut tasks).await;

        Ok(())
    }

    /// Handle a single connection.
    pub async fn handle_connection(
        socket: &mut (impl AsyncRead + AsyncWrite + Unpin),
        routes: Arc<RwLock<Vec<Route>>>,
        read_buffer_size: usize,
    ) -> Result<(), Error> {
        let mut buf = vec![0; read_buffer_size];

        // Read data from the socket
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            return Ok(()); // Connection closed
        }

        // Parse the HTTP request
        let request = match crate::parse_request(&buf[..n]) {
            Ok(req) => req,
            Err(e) => {
                let response = HttpResponse::new(StatusCode::BadRequest)
                    .with_content_type("text/plain")
                    .with_body_string(format!("Error parsing request: {}", e));
                socket.write_all(&response.to_bytes()).await?;
                return Err(Error::ParseError(e));
            }
        };

        // Find a matching route
        let routes_guard = routes.read().await;
        let matching_routes: Vec<&Route> = routes_guard
            .iter()
            .filter(|route| route.path == request.path)
            .collect();

        if matching_routes.is_empty() {
            let response = HttpResponse::new(StatusCode::NotFound)
                .with_content_type("text/plain")
                .with_body_string(format!("Not found: {}", request.path));
            socket.write_all(&response.to_bytes()).await?;
            return Err(Error::NotFound(request.path));
        }

        // Find a route that matches the method
        let route = matching_routes
            .iter()
            .find(|route| route.methods.contains(&request.method));

        match route {
            Some(route) => {
                // Call the handler
                let response = match (route.handler)(request).await {
                    Ok(resp) => resp,
                    Err(e) => {
                        let response = HttpResponse::new(StatusCode::InternalServerError)
                            .with_content_type("text/plain")
                            .with_body_string(format!("Internal server error: {}", e));
                        socket.write_all(&response.to_bytes()).await?;
                        return Err(e);
                    }
                };

                // Send the response
                socket.write_all(&response.to_bytes()).await?;
            }
            None => {
                // Method not allowed
                let allowed_methods: Vec<String> = matching_routes
                    .iter()
                    .flat_map(|route| route.methods.iter().map(|m| m.to_string()))
                    .collect();

                let response = HttpResponse::new(StatusCode::MethodNotAllowed)
                    .with_header("Allow", allowed_methods.join(", "))
                    .with_content_type("text/plain")
                    .with_body_string(format!(
                        "Method {} not allowed for path: {}. Allowed methods: {}",
                        request.method,
                        request.path,
                        allowed_methods.join(", ")
                    ));

                socket.write_all(&response.to_bytes()).await?;
                return Err(Error::MethodNotAllowed(request.method, request.path));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::sync::{mpsc, Semaphore};
    use tokio::task::JoinSet;
    use tokio::time;
    use log::debug;

    #[test]
    fn test_status_code_reason_phrase() {
        assert_eq!(StatusCode::Ok.reason_phrase(), "OK");
        assert_eq!(StatusCode::NotFound.reason_phrase(), "Not Found");
        assert_eq!(
            StatusCode::InternalServerError.reason_phrase(),
            "Internal Server Error"
        );
    }

    #[tokio::test]
    async fn test_connection_limiting() {
        // Create a semaphore with a small limit
        let max_connections = 2;
        let semaphore = Arc::new(Semaphore::new(max_connections));
        let active_connections = Arc::new(AtomicUsize::new(0));

        // Create a mock function that simulates handling a connection
        async fn handle_connection(
            semaphore: Arc<Semaphore>,
            active_connections: Arc<AtomicUsize>,
            connection_id: usize,
        ) -> Result<(), String> {
            // Try to acquire a permit
            let permit = match semaphore.clone().try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    return Err(format!(
                        "Connection {} rejected: limit reached",
                        connection_id
                    ));
                }
            };

            // Increment active connections counter
            let count = active_connections.fetch_add(1, Ordering::SeqCst) + 1;
            debug!(
                "Connection {} accepted. Active connections: {}",
                connection_id, count
            );

            // Simulate some work
            tokio::time::sleep(Duration::from_millis(10)).await;

            // Decrement active connections counter (permit is dropped when this function returns)
            let count = active_connections.fetch_sub(1, Ordering::SeqCst) - 1;
            debug!(
                "Connection {} completed. Active connections: {}",
                connection_id, count
            );

            // The permit is dropped here, releasing the semaphore slot
            drop(permit);

            Ok(())
        }

        // Spawn multiple concurrent connections
        let mut handles = vec![];
        let mut results = vec![];

        // First, spawn max_connections tasks that should succeed
        for i in 0..max_connections {
            let semaphore_clone = semaphore.clone();
            let active_clone = active_connections.clone();
            let handle =
                tokio::spawn(
                    async move { handle_connection(semaphore_clone, active_clone, i).await },
                );
            handles.push(handle);
        }

        // Wait a bit to ensure the first connections are being processed
        tokio::time::sleep(Duration::from_millis(5)).await;

        // Now spawn one more connection that should be rejected
        let semaphore_clone = semaphore.clone();
        let active_clone = active_connections.clone();
        let reject_handle = tokio::spawn(async move {
            handle_connection(semaphore_clone, active_clone, max_connections).await
        });

        // Wait for all connections to complete
        for handle in handles {
            results.push(handle.await.unwrap());
        }

        // Check the result of the connection that should be rejected
        let reject_result = reject_handle.await.unwrap();

        // Verify that all initial connections succeeded
        for (i, result) in results.iter().enumerate() {
            assert!(result.is_ok(), "Connection {} should have succeeded", i);
        }

        // Verify that the extra connection was rejected
        assert!(
            reject_result.is_err(),
            "Connection {} should have been rejected",
            max_connections
        );
        assert!(
            reject_result.unwrap_err().contains("limit reached"),
            "Rejection message should indicate limit reached"
        );

        // Verify that no active connections remain
        assert_eq!(
            active_connections.load(Ordering::SeqCst),
            0,
            "All connections should be completed"
        );
    }

    #[tokio::test]
    async fn test_server_config_max_connections() {
        // Create a server configuration with a custom max_connections value
        let custom_max_connections = 42;
        let config = ServerConfig {
            addr: "127.0.0.1:8080".parse().unwrap(),
            max_connections: custom_max_connections,
            read_buffer_size: 4096,
        };

        // Create a server with the custom configuration
        let server = HttpServer::new(config);

        // Verify that the server's config has the correct max_connections value
        assert_eq!(server.config.max_connections, custom_max_connections);

        // Create a different server with the default configuration
        let default_server = HttpServer::new(ServerConfig::default());

        // Verify that the default server's config has the default max_connections value
        assert_eq!(default_server.config.max_connections, 1024);

        // Verify that the two servers have different max_connections values
        assert_ne!(
            server.config.max_connections,
            default_server.config.max_connections
        );
    }

    #[test]
    fn test_http_response_creation() {
        let response = HttpResponse::new(StatusCode::Ok);
        assert_eq!(response.status, StatusCode::Ok);
        assert_eq!(
            response.headers.get("Server"),
            Some(&"microhttp-rs".to_string())
        );
        assert!(response.body.is_empty());
    }

    #[test]
    fn test_http_response_with_body_string() {
        let body = "Hello, world!";
        let response = HttpResponse::new(StatusCode::Ok).with_body_string(body);

        assert_eq!(response.body, body.as_bytes());
        assert_eq!(
            response.headers.get("Content-Length"),
            Some(&body.len().to_string())
        );
    }

    #[test]
    fn test_http_response_with_body_bytes() {
        let body = b"Binary data";
        let response = HttpResponse::new(StatusCode::Ok).with_body_bytes(body.to_vec());

        assert_eq!(response.body, body);
        assert_eq!(
            response.headers.get("Content-Length"),
            Some(&body.len().to_string())
        );
    }

    #[test]
    fn test_http_response_with_header() {
        let response = HttpResponse::new(StatusCode::Ok).with_header("X-Custom", "Value");

        assert_eq!(response.headers.get("X-Custom"), Some(&"Value".to_string()));
    }

    #[test]
    fn test_http_response_with_content_type() {
        let response = HttpResponse::new(StatusCode::Ok).with_content_type("application/json");

        assert_eq!(
            response.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_http_response_to_bytes() {
        let response = HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_body_string("Hello, world!");

        let bytes = response.to_bytes();
        let response_str = String::from_utf8_lossy(&bytes);

        assert!(response_str.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response_str.contains("Content-Type: text/plain\r\n"));
        assert!(response_str.contains("Content-Length: 13\r\n"));
        assert!(response_str.contains("Server: microhttp-rs\r\n"));
        assert!(response_str.ends_with("\r\n\r\nHello, world!"));
    }

    #[test]
    fn test_http_response_with_json() {
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestUser {
            name: String,
            age: u32,
        }

        let user = TestUser {
            name: "Jane Doe".to_string(),
            age: 25,
        };

        // Test with_json method
        let response = HttpResponse::new(StatusCode::Ok).with_json(&user).unwrap();

        // Verify content type is set to application/json
        assert_eq!(
            response.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );

        // Verify the body contains the serialized JSON
        let expected_json = serde_json::to_vec(&user).unwrap();
        assert_eq!(response.body, expected_json);

        // Verify the Content-Length header is set correctly
        assert_eq!(
            response.headers.get("Content-Length"),
            Some(&expected_json.len().to_string())
        );

        // Deserialize the body back to verify it's valid JSON
        let deserialized: TestUser = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(deserialized, user);
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        // Create a channel for shutdown signaling
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Create a flag to track if shutdown was received
        let shutdown_received = Arc::new(AtomicBool::new(false));
        let shutdown_received_clone = shutdown_received.clone();

        // Spawn a task that simulates the server loop
        let server_handle = tokio::spawn(async move {
            // Create a JoinSet to track tasks
            let mut tasks = JoinSet::new();

            // Spawn a few "connection handler" tasks
            for i in 0..3 {
                tasks.spawn(async move {
                    // Simulate some work
                    time::sleep(Duration::from_millis(50)).await;
                    debug!("Task {} completed", i);
                    Ok::<_, Error>(())
                });
            }

            // Wait for shutdown signal or timeout
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    shutdown_received_clone.store(true, Ordering::SeqCst);
                    debug!("Shutdown signal received");
                }
                _ = time::sleep(Duration::from_secs(5)) => {
                    panic!("Test timed out waiting for shutdown signal");
                }
            }

            // Wait for all tasks to complete
            while let Some(res) = tasks.join_next().await {
                assert!(res.is_ok(), "Task failed: {:?}", res);
            }

            debug!("All tasks completed after shutdown");
        });

        // Wait a bit for the server to start
        time::sleep(Duration::from_millis(10)).await;

        // Send shutdown signal
        shutdown_tx.send(()).await.expect("Failed to send shutdown signal");

        // Wait for the server to shut down
        server_handle.await.expect("Server task failed");

        // Verify that shutdown was received
        assert!(shutdown_received.load(Ordering::SeqCst), "Shutdown signal was not received");
    }

    #[tokio::test]
    async fn test_graceful_shutdown_with_active_connections() {
        // Create a channel for shutdown signaling
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        // Create counters to track active and completed connections
        let active_connections = Arc::new(AtomicUsize::new(0));
        let completed_connections = Arc::new(AtomicUsize::new(0));
        let active_clone = active_connections.clone();
        let completed_clone = completed_connections.clone();

        // Create a flag to track if shutdown was received
        let shutdown_received = Arc::new(AtomicBool::new(false));
        let shutdown_received_clone = shutdown_received.clone();

        // Spawn a task that simulates the server loop
        let server_handle = tokio::spawn(async move {
            // Create a JoinSet to track tasks
            let mut tasks = JoinSet::new();

            // Spawn "connection handler" tasks with different durations
            for i in 0..5 {
                let active = active_clone.clone();
                let completed = completed_clone.clone();

                tasks.spawn(async move {
                    // Increment active connections
                    active.fetch_add(1, Ordering::SeqCst);

                    // Simulate work with different durations
                    let duration = Duration::from_millis(50 * (i + 1));
                    time::sleep(duration).await;

                    // Decrement active and increment completed
                    active.fetch_sub(1, Ordering::SeqCst);
                    completed.fetch_add(1, Ordering::SeqCst);

                    debug!("Task {} completed after {:?}", i, duration);
                    Ok::<_, Error>(())
                });
            }

            // Wait for shutdown signal or timeout
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    shutdown_received_clone.store(true, Ordering::SeqCst);
                    debug!("Shutdown signal received, waiting for tasks to complete");
                }
                _ = time::sleep(Duration::from_secs(5)) => {
                    panic!("Test timed out waiting for shutdown signal");
                }
            }

            // Wait for all tasks to complete
            while let Some(res) = tasks.join_next().await {
                assert!(res.is_ok(), "Task failed: {:?}", res);
            }

            debug!("All tasks completed after shutdown");
        });

        // Wait a bit for the server to start and some tasks to begin
        time::sleep(Duration::from_millis(75)).await;

        // Verify that some connections are active
        let active_before_shutdown = active_connections.load(Ordering::SeqCst);
        let completed_before_shutdown = completed_connections.load(Ordering::SeqCst);
        assert!(active_before_shutdown > 0, "No active connections before shutdown");

        // Send shutdown signal
        shutdown_tx.send(()).await.expect("Failed to send shutdown signal");

        // Wait for the server to shut down
        server_handle.await.expect("Server task failed");

        // Verify that shutdown was received
        assert!(shutdown_received.load(Ordering::SeqCst), "Shutdown signal was not received");

        // Verify that all connections were completed
        assert_eq!(active_connections.load(Ordering::SeqCst), 0, "Not all connections completed");
        assert_eq!(completed_connections.load(Ordering::SeqCst), 5, "Not all connections were processed");
        assert!(completed_connections.load(Ordering::SeqCst) > completed_before_shutdown, 
                "No additional connections completed after shutdown");
    }
}

//! HTTP server implementation.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{RwLock, mpsc};
use tokio::task::JoinSet;
use tokio::signal;
use log::{info, warn, error};
use std::net::SocketAddr;

use crate::parser::{HttpRequest, Method, parse_request};
use crate::server::config::ServerConfig;
use crate::server::error::Error;
use crate::server::handler::Route;
use crate::server::response::{HttpResponse, StatusCode};

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
        let banner = include_str!("../banner.txt");
        info!("\n{banner}");

        // Display registered endpoints
        let routes = self.routes.read().await;
        info!("Registered endpoints:");
        for route in routes.iter() {
            let methods = route.methods.iter()
                .map(|m| format!("{m}"))
                .collect::<Vec<String>>()
                .join(", ");
            info!("  {methods} {}", route.path);
        }

        Ok(())
    }

    /// Set up the TCP listener.
    async fn setup_listener(&self) -> Result<TcpListener, Error> {
        let listener = TcpListener::bind(&self.config.addr).await?;
        info!("Server listening on http://{addr}", addr = self.config.addr);
        Ok(listener)
    }

    /// Set up a Ctrl+C handler for graceful shutdown.
    fn setup_ctrl_c_handler(shutdown_tx: Arc<mpsc::Sender<()>>, tasks: &mut JoinSet<()>) {
        // Spawn a task to handle Ctrl+C
        tasks.spawn(async move {
            match signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received Ctrl+C, initiating graceful shutdown");
                    let _ = shutdown_tx.send(()).await;
                }
                Err(e) => {
                    error!("Error setting up Ctrl+C handler: {e}");
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
                warn!("Connection limit reached, rejecting connection from {addr}");
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
                error!("Error handling connection: {e}");

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
        error!("Error accepting connection: {e}");

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
        info!("Waiting for {len} active connections to complete...", len = tasks.len());
        let shutdown_timeout = tokio::time::Duration::from_secs(30);
        let _ = tokio::time::timeout(shutdown_timeout, async {
            while let Some(res) = tasks.join_next().await {
                if let Err(e) = res {
                    error!("Task failed during shutdown: {e}");
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
        let request = match parse_request(&buf[..n]) {
            Ok(req) => req,
            Err(e) => {
                let response = HttpResponse::new(StatusCode::BadRequest)
                    .with_content_type("text/plain")
                    .with_body_string(format!("Error parsing request: {e}"));
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
                .with_body_string(format!("Not found: {path}", path = request.path));
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
                            .with_body_string(format!("Internal server error: {e}"));
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
                        "Method {method} not allowed for path: {path}. Allowed methods: {allowed}",
                        method = request.method,
                        path = request.path,
                        allowed = allowed_methods.join(", ")
                    ));

                socket.write_all(&response.to_bytes()).await?;
                return Err(Error::MethodNotAllowed(request.method, request.path));
            }
        }

        Ok(())
    }
}
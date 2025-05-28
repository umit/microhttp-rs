//! A basic HTTP server example demonstrating the core features of the microhttp-rs server API.

use microhttp_rs::{HttpResponse, HttpServer, Method, ServerConfig, StatusCode};
use log::{info};
use env_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();
    
    // Create a server configuration with default values
    let config = ServerConfig {
        addr: "127.0.0.1:8081".parse()?,
        max_connections: 100,
        read_buffer_size: 4096,
    };

    // Create a new HTTP server
    let server = HttpServer::new(config);

    // Add a simple route that responds with "Hello, World!"
    server.add_route("/", vec![Method::GET], |_req| async move {
        Ok(HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_body_string("Hello, World!"))
    }).await;

    // Add a route that handles query parameters
    server.add_route("/hello", vec![Method::GET], |req| async move {
        // Get the 'name' query parameter if it exists
        let name = req.get_query_param("name").map_or("World", |s| s.as_str());

        Ok(HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_body_string(format!("Hello, {}!", name)))
    }).await;

    // Add a route that returns different status codes
    server.add_route("/status", vec![Method::GET], |req| async move {
        // Get the 'code' query parameter if it exists
        let status_code = match req.get_query_param("code").map(|s| s.as_str()) {
            Some("404") => StatusCode::NotFound,
            Some("500") => StatusCode::InternalServerError,
            _ => StatusCode::Ok,
        };

        Ok(HttpResponse::new(status_code)
            .with_content_type("text/plain")
            .with_body_string(format!("Status: {}", status_code as u16)))
    }).await;

    info!("Server configured with the following routes:");
    info!("  GET  /");
    info!("  GET  /hello");
    info!("  GET  /status");
    info!("Starting server on http://127.0.0.1:8081");

    // Start the server
    server.start().await?;

    Ok(())
}
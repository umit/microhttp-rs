//! A more advanced HTTP server example demonstrating the microhttp-rs server API.

use microhttp_rs::{
    HttpResponse, HttpServer, Method, ServerConfig, StatusCode, ServerError
};
use serde::{Deserialize, Serialize};
use log::{info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();
    // Create a server configuration
    let config = ServerConfig {
        addr: "127.0.0.1:8083".parse()?,
        max_connections: 1024,
        read_buffer_size: 8192,
    };

    // Create a new HTTP server
    let server = HttpServer::new(config);

    // Add routes

    // 1. Simple GET route
    server.add_route("/", vec![Method::GET], |_req| async move {
        Ok(HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/html")
            .with_body_string("<html><body><h1>Welcome to microhttp-rs!</h1></body></html>"))
    }).await;

    // 2. Route with path parameter
    server.add_route("/hello", vec![Method::GET], |req| async move {
        // Get the 'name' query parameter if it exists
        let name = req.get_query_param("name").map_or("World", |s| s.as_str());

        Ok(HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_body_string(format!("Hello, {name}!")))
    }).await;

    // Define data structures for JSON
    #[derive(Debug, Serialize, Deserialize)]
    struct Message {
        message: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct User {
        name: String,
        email: String,
        age: Option<u32>,
    }

    // 3. Route that handles multiple HTTP methods with JSON
    server.add_route("/api/data", vec![Method::GET, Method::POST], |req| async move {
        match req.method {
            Method::GET => {
                // Return some data as JSON
                let data = Message {
                    message: "This is GET data".to_string(),
                };

                HttpResponse::new(StatusCode::Ok)
                    .with_json(&data)
                    .map_err(|e| ServerError::InternalError(format!("JSON error: {e}")))
            },
            Method::POST => {
                // Process the request and return a response
                let data = Message {
                    message: "Data created successfully".to_string(),
                };

                HttpResponse::new(StatusCode::Created)
                    .with_json(&data)
                    .map_err(|e| ServerError::InternalError(format!("JSON error: {e}")))
            },
            _ => Err(ServerError::InternalError("Unexpected method".to_string())),
        }
    }).await;

    // 4. Route that demonstrates JSON request parsing
    server.add_route("/api/users", vec![Method::POST], |req| async move {
        // Check if the request is JSON
        if !req.is_json() {
            return Ok(HttpResponse::new(StatusCode::BadRequest)
                .with_content_type("text/plain")
                .with_body_string("Content-Type must be application/json"));
        }

        // Parse the JSON body
        match req.json::<User>() {
            Ok(user) => {
                // Create a response with the user data
                let response = Message {
                    message: format!("User {} created successfully", user.name),
                };

                HttpResponse::new(StatusCode::Created)
                    .with_json(&response)
                    .map_err(|e| ServerError::InternalError(format!("JSON error: {e}")))
            },
            Err(e) => {
                Ok(HttpResponse::new(StatusCode::BadRequest)
                    .with_content_type("text/plain")
                    .with_body_string(format!("Invalid JSON: {e}")))
            }
        }
    }).await;

    // 5. Route that returns different status codes
    server.add_route("/status", vec![Method::GET], |req| async move {
        // Get the 'code' query parameter if it exists
        let status_code = match req.get_query_param("code").map(|s| s.as_str()) {
            Some("200") => StatusCode::Ok,
            Some("201") => StatusCode::Created,
            Some("400") => StatusCode::BadRequest,
            Some("404") => StatusCode::NotFound,
            Some("500") => StatusCode::InternalServerError,
            _ => StatusCode::Ok,
        };

        Ok(HttpResponse::new(status_code)
            .with_content_type("text/plain")
            .with_body_string(format!("Status: {}", status_code as u16)))
    }).await;

    // 6. Route that demonstrates headers
    server.add_route("/headers", vec![Method::GET], |req| async move {
        let mut response_body = String::from("Request Headers:\n\n");

        for (name, value) in &req.headers {
            response_body.push_str(&format!("{name}: {value}\n"));
        }

        Ok(HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_header("X-Custom-Header", "Custom Value")
            .with_body_string(response_body))
    }).await;

    info!("Server configured with the following routes:");
    info!("  GET  /");
    info!("  GET  /hello");
    info!("  GET  /api/data");
    info!("  POST /api/data");
    info!("  POST /api/users");
    info!("  GET  /status");
    info!("  GET  /headers");
    info!("Starting server on http://127.0.0.1:8083");

    // Start the server
    server.start().await?;

    Ok(())
}

//! A more advanced HTTP server example demonstrating the microhttp-rs server API.

use microhttp_rs::{
    HttpResponse, HttpServer, Method, ServerConfig, StatusCode, ServerError
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a server configuration
    let config = ServerConfig {
        addr: "127.0.0.1:8080".parse()?,
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
        let name = if let Some(query) = req.path.split_once('?') {
            if let Some(name_param) = query.1.split('&')
                .find_map(|param| {
                    let parts: Vec<&str> = param.split('=').collect();
                    if parts.len() == 2 && parts[0] == "name" {
                        Some(parts[1])
                    } else {
                        None
                    }
                }) {
                name_param
            } else {
                "World"
            }
        } else {
            "World"
        };

        Ok(HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_body_string(format!("Hello, {}!", name)))
    }).await;

    // 3. Route that handles multiple HTTP methods
    server.add_route("/api/data", vec![Method::GET, Method::POST], |req| async move {
        match req.method {
            Method::GET => {
                // Return some data
                Ok(HttpResponse::new(StatusCode::Ok)
                    .with_content_type("application/json")
                    .with_body_string(r#"{"message": "This is GET data"}"#))
            },
            Method::POST => {
                // Process the request and return a response
                Ok(HttpResponse::new(StatusCode::Created)
                    .with_content_type("application/json")
                    .with_body_string(r#"{"message": "Data created successfully"}"#))
            },
            _ => Err(ServerError::InternalError("Unexpected method".to_string())),
        }
    }).await;

    // 4. Route that returns different status codes
    server.add_route("/status", vec![Method::GET], |req| async move {
        // Get the 'code' query parameter if it exists
        let status_code = if let Some(query) = req.path.split_once('?') {
            if let Some(code_str) = query.1.split('&')
                .find_map(|param| {
                    let parts: Vec<&str> = param.split('=').collect();
                    if parts.len() == 2 && parts[0] == "code" {
                        Some(parts[1])
                    } else {
                        None
                    }
                }) {
                match code_str {
                    "200" => StatusCode::Ok,
                    "201" => StatusCode::Created,
                    "400" => StatusCode::BadRequest,
                    "404" => StatusCode::NotFound,
                    "500" => StatusCode::InternalServerError,
                    _ => StatusCode::Ok,
                }
            } else {
                StatusCode::Ok
            }
        } else {
            StatusCode::Ok
        };

        Ok(HttpResponse::new(status_code)
            .with_content_type("text/plain")
            .with_body_string(format!("Status: {}", status_code as u16)))
    }).await;

    // 5. Route that demonstrates headers
    server.add_route("/headers", vec![Method::GET], |req| async move {
        let mut response_body = String::from("Request Headers:\n\n");

        for (name, value) in &req.headers {
            response_body.push_str(&format!("{}: {}\n", name, value));
        }

        Ok(HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_header("X-Custom-Header", "Custom Value")
            .with_body_string(response_body))
    }).await;

    println!("Server configured with the following routes:");
    println!("  GET  /");
    println!("  GET  /hello");
    println!("  GET  /api/data");
    println!("  POST /api/data");
    println!("  GET  /status");
    println!("  GET  /headers");
    println!("\nStarting server on http://127.0.0.1:8080");

    // Start the server
    server.start().await?;

    Ok(())
}

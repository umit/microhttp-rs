# microhttp-rs

A minimal HTTP parser and server library written in Rust. This library provides both a lightweight HTTP request parser and a fully-featured HTTP server implementation.

## Features

- Parse HTTP requests from byte slices
- Support for common HTTP methods (GET, POST, PUT, DELETE, HEAD, OPTIONS, PATCH)
- Support for HTTP versions 1.0, 1.1, and 2.0
- Proper error handling with descriptive error messages
- Built-in HTTP server with:
  - Async/await support using Tokio
  - Route registration with method filtering
  - Query parameter parsing
  - JSON request and response handling
  - Custom header support
  - Configurable connection limits and buffer sizes
  - Graceful shutdown handling

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
microhttp-rs = "0.1.0"
tokio = { version = "1.36.0", features = ["rt", "rt-multi-thread", "io-util", "net", "sync", "macros", "time", "signal"] }
```

### Example: Parsing an HTTP request

```rust
use microhttp_rs::{parse_request, HttpRequest, Method, HttpVersion, Error};

fn main() {
    let request_bytes = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";

    match parse_request(request_bytes) {
        Ok(request) => {
            println!("Method: {}", request.method);
            println!("Path: {}", request.path);
            println!("Version: {}", request.version);
            println!("Headers: {:?}", request.headers);
        }
        Err(err) => {
            println!("Error parsing request: {}", err);
        }
    }
}
```

### Example: Using the HTTP Server API

The library provides a built-in HTTP server with routing capabilities. Here's a basic example:

```rust
use microhttp_rs::{HttpResponse, HttpServer, Method, ServerConfig, StatusCode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Create a server configuration
    let config = ServerConfig {
        addr: "127.0.0.1:8080".parse()?,  // Server address and port
        max_connections: 100,             // Maximum concurrent connections
        read_buffer_size: 4096,           // Buffer size for reading requests
    };

    // Step 2: Create a new HTTP server
    let server = HttpServer::new(config);

    // Step 3: Add routes to handle different paths
    // Simple route that returns "Hello, World!"
    server.add_route("/", vec![Method::GET], |_req| async move {
        Ok(HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_body_string("Hello, World!"))
    }).await;

    // Route that handles query parameters
    server.add_route("/hello", vec![Method::GET], |req| async move {
        // Get the 'name' query parameter if it exists
        let name = req.get_query_param("name").map_or("World", |s| s.as_str());

        Ok(HttpResponse::new(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_body_string(format!("Hello, {}!", name)))
    }).await;

    // Step 4: Start the server
    server.start().await?;

    Ok(())
}
```

This example demonstrates:
1. Creating a server configuration
2. Creating a new HTTP server instance
3. Adding routes to handle different paths and HTTP methods
4. Processing query parameters
5. Starting the server

For more advanced examples, see the `examples` directory in the repository.

### Example: JSON Handling

The server supports JSON requests and responses:

```rust
use microhttp_rs::{HttpResponse, HttpServer, Method, ServerConfig, StatusCode, ServerError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
}

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

    // Add a route that handles JSON
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
                HttpResponse::new(StatusCode::Created)
                    .with_json(&user)
                    .map_err(|e| ServerError::InternalError(format!("JSON error: {}", e)))
            },
            Err(e) => {
                Ok(HttpResponse::new(StatusCode::BadRequest)
                    .with_content_type("text/plain")
                    .with_body_string(format!("Invalid JSON: {}", e)))
            }
        }
    }).await;

    // Start the server
    server.start().await?;

    Ok(())
}
```

## Examples

The library includes several examples in the `examples` directory:

### Simple Parser

A basic example showing how to parse HTTP requests:

```bash
cargo run --example simple_parser
```

### Simple HTTP Server

A low-level HTTP server example using the parser directly:

```bash
cargo run --example simple_server
```

Then visit http://localhost:8082 in your browser or use curl:

```bash
curl http://localhost:8082/hello
```

### Basic HTTP Server

A basic example showing how to use the HTTP server API with minimal configuration:

```bash
cargo run --example basic_server
```

This example demonstrates:
- Setting up a server with basic configuration
- Adding simple routes
- Handling query parameters
- Returning different status codes

You can test it with:

```bash
# Get a simple response
curl http://localhost:8081/

# Test the hello endpoint with a query parameter
curl "http://localhost:8081/hello?name=YourName"

# Test different status codes
curl "http://localhost:8081/status?code=404"
```

### Advanced HTTP Server

A more comprehensive HTTP server example demonstrating routing, JSON handling, query parameters, and more:

```bash
cargo run --example http_server
```

This example includes:
- Multiple routes with different HTTP methods
- Query parameter handling
- JSON request and response processing
- Custom headers
- Different status codes

You can test it with:

```bash
# Test the hello endpoint with a query parameter
curl "http://localhost:8083/hello?name=YourName"

# Test the JSON API
curl -X POST -H "Content-Type: application/json" -d '{"name":"John","email":"john@example.com"}' http://localhost:8083/api/users

# Test different status codes
curl "http://localhost:8083/status?code=404"

# View request headers
curl http://localhost:8083/headers
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

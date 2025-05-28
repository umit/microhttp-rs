//! A simple example demonstrating how to use the microhttp-rs library to parse HTTP requests.

use microhttp_rs::parse_request;
use log::{info, error};
use env_logger;

fn main() {
    // Initialize the logger
    env_logger::init();
    // Example HTTP request
    let request_bytes =
        b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: ExampleBrowser/1.0\r\n\r\n";

    // Parse the request
    match parse_request(request_bytes) {
        Ok(request) => {
            info!("Successfully parsed HTTP request:");
            info!("Method: {}", request.method);
            info!("Path: {}", request.path);
            info!("Version: {}", request.version);
            info!("Headers:");
            for (name, value) in &request.headers {
                info!("  {}: {}", name, value);
            }
        }
        Err(err) => {
            error!("Error parsing request: {}", err);
        }
    }

    // Example with an invalid request
    let invalid_request = b"INVALID /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";

    match parse_request(invalid_request) {
        Ok(_) => {
            error!("Unexpectedly parsed invalid request!");
        }
        Err(err) => {
            info!("Expected error parsing invalid request: {}", err);
        }
    }
}

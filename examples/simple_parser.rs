//! A simple example demonstrating how to use the microhttp-rs library to parse HTTP requests.

use microhttp_rs::parse_request;

fn main() {
    // Example HTTP request
    let request_bytes =
        b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: ExampleBrowser/1.0\r\n\r\n";

    // Parse the request
    match parse_request(request_bytes) {
        Ok(request) => {
            println!("Successfully parsed HTTP request:");
            println!("Method: {}", request.method);
            println!("Path: {}", request.path);
            println!("Version: {}", request.version);
            println!("Headers:");
            for (name, value) in &request.headers {
                println!("  {}: {}", name, value);
            }
        }
        Err(err) => {
            println!("Error parsing request: {}", err);
        }
    }

    // Example with an invalid request
    let invalid_request = b"INVALID /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";

    match parse_request(invalid_request) {
        Ok(_) => {
            println!("\nUnexpectedly parsed invalid request!");
        }
        Err(err) => {
            println!("\nExpected error parsing invalid request: {}", err);
        }
    }
}

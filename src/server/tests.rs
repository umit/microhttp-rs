//! Tests for the HTTP server implementation.

#[cfg(test)]
mod server_tests {
    use std::io::{self, Cursor};
    use std::net::SocketAddr;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio::sync::mpsc;

    use crate::parser::{HttpRequest, Method, HttpVersion};
    use crate::server::{HttpServer, ServerConfig, HttpResponse, StatusCode, Error};

    // Mock TcpStream for testing
    struct MockTcpStream {
        read_data: Cursor<Vec<u8>>,
        write_data: Vec<u8>,
    }

    impl MockTcpStream {
        fn new(read_data: Vec<u8>) -> Self {
            Self {
                read_data: Cursor::new(read_data),
                write_data: Vec::new(),
            }
        }

        fn written_data(&self) -> &[u8] {
            &self.write_data
        }
    }

    impl AsyncRead for MockTcpStream {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            let n = std::io::Read::read(&mut this.read_data, buf.initialize_unfilled())?;
            buf.advance(n);
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for MockTcpStream {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            let this = self.get_mut();
            this.write_data.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_server_creation() {
        let config = ServerConfig {
            addr: "127.0.0.1:8080".parse().unwrap(),
            max_connections: 100,
            read_buffer_size: 4096,
        };

        let server = HttpServer::new(config.clone());
        assert_eq!(server.config.addr, config.addr);
        assert_eq!(server.config.max_connections, config.max_connections);
        assert_eq!(server.config.read_buffer_size, config.read_buffer_size);
    }

    #[tokio::test]
    async fn test_add_route() {
        let server = HttpServer::new(ServerConfig::default());
        
        // Add a route
        server.add_route("/test", vec![Method::GET], |_req| async {
            Ok(HttpResponse::new(StatusCode::Ok)
                .with_content_type("text/plain")
                .with_body_string("Test response"))
        }).await;
        
        // Verify the route was added
        let routes = server.routes.read().await;
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/test");
        assert_eq!(routes[0].methods, vec![Method::GET]);
    }

    #[tokio::test]
    async fn test_handle_connection_with_valid_request() {
        // Create a mock request
        let request = b"GET /test HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut stream = MockTcpStream::new(request.to_vec());
        
        // Create a server with a test route
        let server = HttpServer::new(ServerConfig::default());
        server.add_route("/test", vec![Method::GET], |_req| async {
            Ok(HttpResponse::new(StatusCode::Ok)
                .with_content_type("text/plain")
                .with_body_string("Test response"))
        }).await;
        
        // Handle the connection
        let result = HttpServer::handle_connection(
            &mut stream,
            server.routes.clone(),
            1024
        ).await;
        
        // Verify the result
        assert!(result.is_ok());
        
        // Verify the response
        let response = String::from_utf8_lossy(stream.written_data());
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response.contains("Content-Type: text/plain\r\n"));
        assert!(response.contains("Test response"));
    }

    #[tokio::test]
    async fn test_handle_connection_with_not_found() {
        // Create a mock request for a non-existent route
        let request = b"GET /nonexistent HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut stream = MockTcpStream::new(request.to_vec());
        
        // Create a server with a different route
        let server = HttpServer::new(ServerConfig::default());
        server.add_route("/test", vec![Method::GET], |_req| async {
            Ok(HttpResponse::new(StatusCode::Ok)
                .with_content_type("text/plain")
                .with_body_string("Test response"))
        }).await;
        
        // Handle the connection
        let result = HttpServer::handle_connection(
            &mut stream,
            server.routes.clone(),
            1024
        ).await;
        
        // Verify the result is an error
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NotFound(_)));
        
        // Verify the response
        let response = String::from_utf8_lossy(stream.written_data());
        assert!(response.starts_with("HTTP/1.1 404 Not Found\r\n"));
        assert!(response.contains("Not found: /nonexistent"));
    }

    #[tokio::test]
    async fn test_handle_connection_with_method_not_allowed() {
        // Create a mock request with a method not allowed for the route
        let request = b"POST /test HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut stream = MockTcpStream::new(request.to_vec());
        
        // Create a server with a route that only accepts GET
        let server = HttpServer::new(ServerConfig::default());
        server.add_route("/test", vec![Method::GET], |_req| async {
            Ok(HttpResponse::new(StatusCode::Ok)
                .with_content_type("text/plain")
                .with_body_string("Test response"))
        }).await;
        
        // Handle the connection
        let result = HttpServer::handle_connection(
            &mut stream,
            server.routes.clone(),
            1024
        ).await;
        
        // Verify the result is an error
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::MethodNotAllowed(_, _)));
        
        // Verify the response
        let response = String::from_utf8_lossy(stream.written_data());
        assert!(response.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"));
        assert!(response.contains("Method POST not allowed for path: /test"));
        assert!(response.contains("Allow: GET\r\n"));
    }

    #[tokio::test]
    async fn test_handle_connection_with_invalid_request() {
        // Create an invalid mock request
        let request = b"INVALID REQUEST";
        let mut stream = MockTcpStream::new(request.to_vec());
        
        // Create a server
        let server = HttpServer::new(ServerConfig::default());
        
        // Handle the connection
        let result = HttpServer::handle_connection(
            &mut stream,
            server.routes.clone(),
            1024
        ).await;
        
        // Verify the result is an error
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ParseError(_)));
        
        // Verify the response
        let response = String::from_utf8_lossy(stream.written_data());
        assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
        assert!(response.contains("Error parsing request:"));
    }

    #[tokio::test]
    async fn test_multiple_routes() {
        // Create a server with multiple routes
        let server = HttpServer::new(ServerConfig::default());
        
        // Add routes
        server.add_route("/route1", vec![Method::GET], |_req| async {
            Ok(HttpResponse::new(StatusCode::Ok)
                .with_body_string("Route 1"))
        }).await;
        
        server.add_route("/route2", vec![Method::POST], |_req| async {
            Ok(HttpResponse::new(StatusCode::Created)
                .with_body_string("Route 2"))
        }).await;
        
        // Test route 1
        let request1 = b"GET /route1 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut stream1 = MockTcpStream::new(request1.to_vec());
        
        let result1 = HttpServer::handle_connection(
            &mut stream1,
            server.routes.clone(),
            1024
        ).await;
        
        assert!(result1.is_ok());
        let response1 = String::from_utf8_lossy(stream1.written_data());
        assert!(response1.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(response1.contains("Route 1"));
        
        // Test route 2
        let request2 = b"POST /route2 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut stream2 = MockTcpStream::new(request2.to_vec());
        
        let result2 = HttpServer::handle_connection(
            &mut stream2,
            server.routes.clone(),
            1024
        ).await;
        
        assert!(result2.is_ok());
        let response2 = String::from_utf8_lossy(stream2.written_data());
        assert!(response2.starts_with("HTTP/1.1 201 Created\r\n"));
        assert!(response2.contains("Route 2"));
    }

    #[tokio::test]
    async fn test_route_with_multiple_methods() {
        // Create a server with a route that accepts multiple methods
        let server = HttpServer::new(ServerConfig::default());
        
        // Add a route that accepts both GET and POST
        server.add_route("/multi", vec![Method::GET, Method::POST], |req| async move {
            match req.method {
                Method::GET => Ok(HttpResponse::new(StatusCode::Ok)
                    .with_body_string("GET response")),
                Method::POST => Ok(HttpResponse::new(StatusCode::Created)
                    .with_body_string("POST response")),
                _ => Err(Error::InternalError("Unexpected method".to_string())),
            }
        }).await;
        
        // Test GET request
        let get_request = b"GET /multi HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut get_stream = MockTcpStream::new(get_request.to_vec());
        
        let get_result = HttpServer::handle_connection(
            &mut get_stream,
            server.routes.clone(),
            1024
        ).await;
        
        assert!(get_result.is_ok());
        let get_response = String::from_utf8_lossy(get_stream.written_data());
        assert!(get_response.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(get_response.contains("GET response"));
        
        // Test POST request
        let post_request = b"POST /multi HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut post_stream = MockTcpStream::new(post_request.to_vec());
        
        let post_result = HttpServer::handle_connection(
            &mut post_stream,
            server.routes.clone(),
            1024
        ).await;
        
        assert!(post_result.is_ok());
        let post_response = String::from_utf8_lossy(post_stream.written_data());
        assert!(post_response.starts_with("HTTP/1.1 201 Created\r\n"));
        assert!(post_response.contains("POST response"));
    }
}
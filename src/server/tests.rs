//! Tests for the HTTP server implementation.

#[cfg(test)]
mod server_tests {
    use std::io::{self, Cursor};
    use std::net::SocketAddr;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio::sync::{mpsc, Semaphore};
    use tokio::task::JoinSet;
    use tokio::time;
    use log::{debug};

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
    #[tokio::test]
    async fn test_connection_limiting() {
        use tokio::sync::Semaphore;
        use std::sync::atomic::{AtomicUsize, Ordering};

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
                    return Err(format!("Connection {} rejected: limit reached", connection_id));
                }
            };

            // Increment active connections counter
            let count = active_connections.fetch_add(1, Ordering::SeqCst) + 1;
            debug!("Connection {} accepted. Active connections: {}", connection_id, count);

            // Simulate some work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Decrement active connections counter (permit is dropped when this function returns)
            let count = active_connections.fetch_sub(1, Ordering::SeqCst) - 1;
            debug!("Connection {} completed. Active connections: {}", connection_id, count);

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
            let handle = tokio::spawn(async move {
                handle_connection(semaphore_clone, active_clone, i).await
            });
            handles.push(handle);
        }

        // Wait a bit to ensure the first connections are being processed
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;

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
        assert!(reject_result.is_err(), "Connection {} should have been rejected", max_connections);
        assert!(reject_result.unwrap_err().contains("limit reached"), 
                "Rejection message should indicate limit reached");

        // Verify that no active connections remain
        assert_eq!(active_connections.load(Ordering::SeqCst), 0, 
                   "All connections should be completed");
    }

    #[tokio::test]
    async fn test_server_connection_limit_response() {
        use tokio::sync::Semaphore;

        // Create a mock function that simulates the server's connection handling
        async fn handle_connection_limit_exceeded(
            socket: &mut MockTcpStream,
        ) {
            // Send a 503 Service Unavailable response
            let response = HttpResponse::new(StatusCode::ServiceUnavailable)
                .with_content_type("text/plain")
                .with_body_string("Server is at capacity, please try again later");

            let _ = socket.write_all(&response.to_bytes()).await;
        }

        // Create a mock socket
        let mut socket = MockTcpStream::new(Vec::new());

        // Simulate handling a connection when the limit is exceeded
        handle_connection_limit_exceeded(&mut socket).await;

        // Verify the response
        let response = String::from_utf8_lossy(socket.written_data());
        assert!(response.starts_with("HTTP/1.1 503 Service Unavailable\r\n"));
        assert!(response.contains("Content-Type: text/plain\r\n"));
        assert!(response.contains("Server is at capacity, please try again later"));
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
        assert_ne!(server.config.max_connections, default_server.config.max_connections);
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

//! Server configuration.

use std::net::SocketAddr;

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
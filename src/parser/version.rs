//! HTTP protocol versions.

use std::fmt;
use std::str::FromStr;

use crate::parser::error::Error;

/// Supported HTTP protocol versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    /// HTTP/1.0: The first version of the HTTP protocol widely used on the web.
    Http10,
    /// HTTP/1.1: Improved version with persistent connections, chunked transfer encoding, and more.
    Http11,
    /// HTTP/2.0: Major revision with multiplexing, header compression, and server push capabilities.
    Http20,
}

impl FromStr for HttpVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HTTP/1.0" => Ok(HttpVersion::Http10),
            "HTTP/1.1" => Ok(HttpVersion::Http11),
            "HTTP/2" | "HTTP/2.0" => Ok(HttpVersion::Http20),
            _ => Err(Error::InvalidVersion(s.to_string())),
        }
    }
}

impl fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpVersion::Http10 => write!(f, "HTTP/1.0"),
            HttpVersion::Http11 => write!(f, "HTTP/1.1"),
            HttpVersion::Http20 => write!(f, "HTTP/2"),
        }
    }
}

//! HTTP request parsing and representation.

use std::collections::HashMap;
use std::str::FromStr;
use serde::de::DeserializeOwned;

use crate::parser::error::Error;
use crate::parser::method::Method;
use crate::parser::version::HttpVersion;

/// Represents an HTTP request.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// The HTTP method (GET, POST, etc.)
    pub method: Method,
    /// The request path
    pub path: String,
    /// The HTTP version
    pub version: HttpVersion,
    /// The HTTP headers
    pub headers: HashMap<String, String>,
    /// The request body
    pub body: Vec<u8>,
    /// Query parameters parsed from the path
    pub query_params: HashMap<String, String>,
}

impl HttpRequest {
    /// Create a new HTTP request.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method
    /// * `path` - The request path
    /// * `version` - The HTTP version
    /// * `headers` - The HTTP headers
    ///
    /// # Returns
    ///
    /// A new HTTP request with an empty body
    pub fn new(method: Method, path: String, version: HttpVersion, headers: HashMap<String, String>) -> Self {
        // Parse query parameters from the path
        let query_params: HashMap<String, String> = path
            .split_once('?')
            .map(|(_, query)| query
                .split('&')
                .filter(|s| !s.is_empty())
                .map(|pair| {
                    if let Some((k, v)) = pair.split_once('=') {
                        (k.to_string(), v.to_string())
                    } else {
                        (pair.to_string(), String::new())
                    }
                })
                .collect())
            .unwrap_or_default();

        Self {
            method,
            path,
            version,
            headers,
            body: Vec::new(),
            query_params,
        }
    }

    /// Create a new HTTP request with a body.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method
    /// * `path` - The request path
    /// * `version` - The HTTP version
    /// * `headers` - The HTTP headers
    /// * `body` - The request body
    ///
    /// # Returns
    ///
    /// A new HTTP request with the specified body
    pub fn with_body(method: Method, path: String, version: HttpVersion, headers: HashMap<String, String>, body: Vec<u8>) -> Self {
        let mut request = Self::new(method, path, version, headers);
        request.body = body;
        request
    }

    /// Get a header value.
    ///
    /// # Arguments
    ///
    /// * `name` - The header name
    ///
    /// # Returns
    ///
    /// The header value, if it exists
    pub fn get_header(&self, name: &str) -> Option<&String> {
        // Headers are case-insensitive, so we need to do a case-insensitive lookup
        self.headers.iter().find_map(|(k, v)| {
            if k.eq_ignore_ascii_case(name) {
                Some(v)
            } else {
                None
            }
        })
    }

    /// Check if a header exists.
    ///
    /// # Arguments
    ///
    /// * `name` - The header name
    ///
    /// # Returns
    ///
    /// true if the header exists, false otherwise
    pub fn has_header(&self, name: &str) -> bool {
        self.get_header(name).is_some()
    }

    /// Parse the request body as JSON.
    ///
    /// # Returns
    ///
    /// The parsed JSON value, or an error if the body is not valid JSON
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, Error> {
        // Check if the Content-Type header is application/json
        if !self.is_json() {
            return Err(Error::MissingHeader("Content-Type: application/json".to_string()));
        }

        // Parse the body as JSON
        let json = serde_json::from_slice(&self.body)?;
        Ok(json)
    }

    /// Check if the request has a JSON body.
    ///
    /// # Returns
    ///
    /// true if the Content-Type header is application/json, false otherwise
    pub fn is_json(&self) -> bool {
        if let Some(content_type) = self.get_header("Content-Type") {
            content_type.starts_with("application/json")
        } else {
            false
        }
    }

    /// Get a query parameter value.
    ///
    /// # Arguments
    ///
    /// * `name` - The query parameter name
    ///
    /// # Returns
    ///
    /// The query parameter value, if it exists
    pub fn get_query_param(&self, name: &str) -> Option<&String> {
        self.query_params.get(name)
    }

    /// Check if a query parameter exists.
    ///
    /// # Arguments
    ///
    /// * `name` - The query parameter name
    ///
    /// # Returns
    ///
    /// true if the query parameter exists, false otherwise
    pub fn has_query_param(&self, name: &str) -> bool {
        self.query_params.contains_key(name)
    }
}

/// Parse an HTTP request from a byte slice.
///
/// # Arguments
///
/// * `input` - A byte slice containing the HTTP request to parse
///
/// # Returns
///
/// The parsed HTTP request, or an error if the request is invalid
pub fn parse_request(input: &[u8]) -> Result<HttpRequest, Error> {
    // Convert the input to a string
    let input_str = match std::str::from_utf8(input) {
        Ok(s) => s,
        Err(_) => return Err(Error::MalformedRequestLine("Invalid UTF-8".to_string())),
    };

    // Split the input into lines
    let mut lines = input_str.lines();

    // Parse the request line
    let request_line = match lines.next() {
        Some(line) => line,
        None => return Err(Error::EmptyRequest),
    };

    // Split the request line into method, path, and version
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() != 3 {
        return Err(Error::MalformedRequestLine(request_line.to_string()));
    }

    // Parse the method
    let method = Method::from_str(parts[0])?;

    // Parse the path
    let path = parts[1].to_string();
    if path.is_empty() {
        return Err(Error::InvalidPath);
    }

    // Parse the version
    let version = HttpVersion::from_str(parts[2])?;

    // Parse the headers
    let mut headers = HashMap::new();
    for line in lines {
        // Empty line indicates the end of headers
        if line.is_empty() {
            break;
        }

        // Split the line into name and value
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(Error::InvalidHeaderFormat);
        }

        // Trim whitespace from the name and value
        let name = parts[0].trim().to_string();
        let value = parts[1].trim().to_string();

        // Add the header to the map
        headers.insert(name, value);
    }

    // Check for required headers
    if version == HttpVersion::Http11 && !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("Host")) {
        return Err(Error::MissingHeader("Host".to_string()));
    }

    // Create the request
    Ok(HttpRequest::new(method, path, version, headers))
}

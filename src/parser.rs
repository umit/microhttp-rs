use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Represents an HTTP request with its components.
///
/// This struct contains all the parsed components of an HTTP request, including
/// the method, path, HTTP version, and headers.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    /// The HTTP method of the request (GET, POST, etc.)
    pub method: Method,

    /// The request path, including any query parameters
    pub path: String,

    /// The HTTP version (1.0, 1.1, 2.0)
    pub version: HttpVersion,

    /// A map of header names (lowercase) to their values
    pub headers: HashMap<String, String>,
}

impl HttpRequest {
    /// Creates a new HTTP request with the given components.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method
    /// * `path` - The request path
    /// * `version` - The HTTP version
    /// * `headers` - A map of headers
    ///
    /// # Returns
    ///
    /// A new HttpRequest instance
    pub fn new(
        method: Method,
        path: String,
        version: HttpVersion,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            method,
            path,
            version,
            headers,
        }
    }

    /// Gets a header value by name (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `name` - The header name (case-insensitive)
    ///
    /// # Returns
    ///
    /// An Option containing the header value if it exists
    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_ascii_lowercase())
    }

    /// Checks if the request has a specific header (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `name` - The header name (case-insensitive)
    ///
    /// # Returns
    ///
    /// true if the header exists, false otherwise
    pub fn has_header(&self, name: &str) -> bool {
        self.headers.contains_key(&name.to_ascii_lowercase())
    }
}

/// HTTP request methods as defined in RFC 7231 and common extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    PATCH,
}

/// Errors that can occur during HTTP request parsing.
#[derive(Debug, Error)]
pub enum Error {
    /// The HTTP method in the request is not supported.
    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(String),

    /// The request path is invalid or missing.
    #[error("Invalid HTTP path")]
    InvalidPath,

    /// The request line is malformed (wrong format or missing components).
    #[error("Malformed request line: {0}")]
    MalformedRequestLine(String),

    /// The HTTP version in the request is not supported.
    #[error("Invalid HTTP version: {0}")]
    InvalidVersion(String),

    /// A required header is missing from the request.
    #[error("Required header is missing: {0}")]
    MissingHeader(String),

    /// A header in the request has an invalid format.
    #[error("Invalid header format")]
    InvalidHeaderFormat,

    /// The request is empty.
    #[error("Empty request")]
    EmptyRequest,
}

// Implement FromStr for Method
impl FromStr for Method {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "HEAD" => Ok(Method::HEAD),
            "OPTIONS" => Ok(Method::OPTIONS),
            "PATCH" => Ok(Method::PATCH),
            _ => Err(Error::InvalidMethod(s.to_string())),
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Supported HTTP protocol versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    Http10,
    Http11,
    Http20,
}

impl FromStr for HttpVersion {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HTTP/1.0" => Ok(HttpVersion::Http10),
            "HTTP/1.1" => Ok(HttpVersion::Http11),
            "HTTP/2" => Ok(HttpVersion::Http20),
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

/// Parse an HTTP request from a byte slice.
///
/// # Arguments
///
/// * `input` - A byte slice containing the HTTP request to parse
///
/// # Returns
///
/// * `Result<HttpRequest, Error>` - The parsed HTTP request or an error
///
/// # Examples
///
/// ```
/// use microhttp_rs::parse_request;
///
/// let request_bytes = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
/// let request = parse_request(request_bytes).unwrap();
///
/// assert_eq!(request.method.to_string(), "GET");
/// assert_eq!(request.path, "/index.html");
/// assert_eq!(request.version.to_string(), "HTTP/1.1");
/// assert_eq!(request.headers.get("host"), Some(&"example.com".to_string()));
/// ```
pub fn parse_request(input: &[u8]) -> Result<HttpRequest, Error> {
    // Check for empty input
    if input.is_empty() {
        return Err(Error::EmptyRequest);
    }

    let input_str = String::from_utf8_lossy(input);

    // Split the input into lines, handling both CRLF and LF line endings
    let lines: Vec<&str> = input_str
        .split(|c| c == '\n' || c == '\r')
        .filter(|s| !s.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(Error::EmptyRequest);
    }

    // Parse the request line
    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    if parts.len() < 3 {
        return Err(Error::MalformedRequestLine(request_line.to_string()));
    }

    // Parse method
    let method = parts[0]
        .parse()
        .map_err(|_| Error::InvalidMethod(parts[0].to_string()))?;

    // Parse path
    let path = parts[1].to_string();

    // Parse version
    let version = parts[2]
        .parse()
        .map_err(|_| Error::InvalidVersion(parts[2].to_string()))?;

    // Parse headers
    let mut headers = HashMap::new();
    for line in lines.iter().skip(1) {
        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        match line.split_once(": ") {
            Some((key, value)) => {
                headers.insert(
                    key.to_ascii_lowercase(), // Headers are case-insensitive
                    value.trim().to_string(),
                );
            }
            None => return Err(Error::InvalidHeaderFormat),
        }
    }

    // Check required headers - Host is only required for HTTP/1.1
    if version == HttpVersion::Http11 && !headers.contains_key("host") {
        return Err(Error::MissingHeader("Host".to_string()));
    }

    Ok(HttpRequest {
        method,
        path,
        version,
        headers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get_request() {
        let input = b"GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_request(input).unwrap();

        assert_eq!(req.method, Method::GET);
        assert_eq!(req.path, "/hello");
        assert_eq!(req.version, HttpVersion::Http11);
        assert_eq!(req.headers.get("host"), Some(&"localhost".to_string()));
    }

    #[test]
    fn test_parse_request_with_multiple_headers() {
        let input = b"POST /submit HTTP/1.1\r\n\
            Host: example.com\r\n\
            Content-Type: application/json\r\n\
            Content-Length: 42\r\n\r\n";

        let req = parse_request(input).unwrap();

        assert_eq!(req.method, Method::POST);
        assert_eq!(req.path, "/submit");
        assert_eq!(req.version, HttpVersion::Http11);
        assert_eq!(req.headers.get("host"), Some(&"example.com".to_string()));
        assert_eq!(
            req.headers.get("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(req.headers.get("content-length"), Some(&"42".to_string()));
    }

    #[test]
    fn test_case_insensitive_headers() {
        let input = b"GET / HTTP/1.1\r\nHoSt: example.com\r\n\r\n";
        let req = parse_request(input).unwrap();

        assert!(req.headers.contains_key("host"));
        assert_eq!(req.headers.get("host"), Some(&"example.com".to_string()));
    }

    #[test]
    fn test_missing_host_header() {
        let input = b"GET /hello HTTP/1.1\r\nContent-Length: 0\r\n\r\n";
        let err = parse_request(input).unwrap_err();

        assert!(matches!(err, Error::MissingHeader(h) if h == "Host"));
    }

    #[test]
    fn test_invalid_method() {
        let input = b"INVALID /hello HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let err = parse_request(input).unwrap_err();

        assert!(matches!(err, Error::InvalidMethod(_)));
    }

    #[test]
    fn test_invalid_http_version() {
        let input = b"GET /hello HTTP/9.9\r\nHost: localhost\r\n\r\n";
        let err = parse_request(input).unwrap_err();

        assert!(matches!(err, Error::InvalidVersion(_)));
    }

    #[test]
    fn test_invalid_header_format() {
        let input = b"GET / HTTP/1.1\r\nInvalidHeader\r\nHost: localhost\r\n\r\n";
        let err = parse_request(input).unwrap_err();

        assert!(matches!(err, Error::InvalidHeaderFormat));
    }

    #[test]
    fn test_empty_request() {
        let input = b"";
        let err = parse_request(input).unwrap_err();

        assert!(matches!(err, Error::EmptyRequest));
    }

    #[test]
    fn test_incomplete_request_line() {
        let input = b"GET\r\nHost: localhost\r\n\r\n";
        let err = parse_request(input).unwrap_err();

        assert!(matches!(err, Error::MalformedRequestLine(_)));
    }

    #[test]
    fn test_http2_version() {
        // Test with HTTP/2.0 format
        let input = b"GET /hello HTTP/2.0\r\nHost: localhost\r\n\r\n";
        let req = parse_request(input).unwrap();
        assert_eq!(req.version, HttpVersion::Http20);

        // Test with HTTP/2 format
        let input = b"GET /hello HTTP/2\r\nHost: localhost\r\n\r\n";
        let req = parse_request(input).unwrap();
        assert_eq!(req.version, HttpVersion::Http20);
    }

    #[test]
    fn test_all_methods() {
        let methods = vec![
            ("GET", Method::GET),
            ("POST", Method::POST),
            ("PUT", Method::PUT),
            ("DELETE", Method::DELETE),
            ("HEAD", Method::HEAD),
            ("OPTIONS", Method::OPTIONS),
            ("PATCH", Method::PATCH),
        ];

        for (method_str, expected_method) in methods {
            let request = format!("{} / HTTP/1.1\r\nHost: localhost\r\n\r\n", method_str);
            let req = parse_request(request.as_bytes()).unwrap();
            assert_eq!(req.method, expected_method);
        }
    }

    #[test]
    fn test_headers_with_multiple_colons() {
        let input = b"GET / HTTP/1.1\r\n\
            Host: localhost:8080\r\n\
            Custom-Header: value: with: colons\r\n\r\n";

        let req = parse_request(input).unwrap();
        assert_eq!(
            req.headers.get("custom-header"),
            Some(&"value: with: colons".to_string())
        );
    }

    #[test]
    fn test_http10_version() {
        let input = b"GET /hello HTTP/1.0\r\nHost: localhost\r\n\r\n";
        let req = parse_request(input).unwrap();

        assert_eq!(req.version, HttpVersion::Http10);
    }

    #[test]
    fn test_http10_without_host() {
        // HTTP/1.0 doesn't require a Host header
        let input = b"GET /hello HTTP/1.0\r\nUser-Agent: test\r\n\r\n";
        let req = parse_request(input).unwrap();

        assert_eq!(req.version, HttpVersion::Http10);
        assert_eq!(req.path, "/hello");
        assert!(!req.headers.contains_key("host"));
    }

    #[test]
    fn test_method_display() {
        assert_eq!(Method::GET.to_string(), "GET");
        assert_eq!(Method::POST.to_string(), "POST");
        assert_eq!(Method::PUT.to_string(), "PUT");
        assert_eq!(Method::DELETE.to_string(), "DELETE");
        assert_eq!(Method::HEAD.to_string(), "HEAD");
        assert_eq!(Method::OPTIONS.to_string(), "OPTIONS");
        assert_eq!(Method::PATCH.to_string(), "PATCH");
    }

    #[test]
    fn test_http_version_display() {
        assert_eq!(HttpVersion::Http10.to_string(), "HTTP/1.0");
        assert_eq!(HttpVersion::Http11.to_string(), "HTTP/1.1");
        assert_eq!(HttpVersion::Http20.to_string(), "HTTP/2");
    }

    #[test]
    fn test_headers_with_trailing_whitespace() {
        let input = b"GET / HTTP/1.1\r\n\
            Host: localhost \r\n\
            Content-Type: text/plain \r\n\r\n";

        let req = parse_request(input).unwrap();
        assert_eq!(req.headers.get("host"), Some(&"localhost".to_string()));
        assert_eq!(
            req.headers.get("content-type"),
            Some(&"text/plain".to_string())
        );
    }

    #[test]
    fn test_mixed_line_endings() {
        let input = b"GET / HTTP/1.1\r\nHost: localhost\nContent-Type: text/plain\r\n\r\n";
        let req = parse_request(input).unwrap();

        assert_eq!(req.method, Method::GET);
        assert_eq!(req.headers.get("host"), Some(&"localhost".to_string()));
        assert_eq!(
            req.headers.get("content-type"),
            Some(&"text/plain".to_string())
        );
    }

    #[test]
    fn test_request_line_with_extra_whitespace() {
        let input = b"GET  /path   HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_request(input).unwrap();

        assert_eq!(req.method, Method::GET);
        assert_eq!(req.path, "/path");
        assert_eq!(req.version, HttpVersion::Http11);
    }

    #[test]
    fn test_empty_path() {
        let input = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_request(input).unwrap();

        assert_eq!(req.path, "/");
    }

    #[test]
    fn test_path_with_query_parameters() {
        let input = b"GET /search?q=rust&page=1 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_request(input).unwrap();

        assert_eq!(req.path, "/search?q=rust&page=1");
    }

    #[test]
    fn test_malformed_utf8_in_request() {
        // Create a request with invalid UTF-8 bytes
        let mut input = Vec::from(*b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
        // Insert an invalid UTF-8 sequence in the middle
        input.splice(5..5, vec![0xFF, 0xFF]);

        // The parser should handle this gracefully using from_utf8_lossy
        let req = parse_request(&input).unwrap();
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.headers.get("host"), Some(&"localhost".to_string()));
    }

    #[test]
    fn test_empty_header_value() {
        let input = b"GET / HTTP/1.1\r\nHost: localhost\r\nX-Empty: \r\n\r\n";
        let req = parse_request(input).unwrap();

        assert_eq!(req.headers.get("x-empty"), Some(&"".to_string()));
    }

    #[test]
    fn test_duplicate_headers() {
        let input = b"GET / HTTP/1.1\r\n\
            Host: first.example.com\r\n\
            Custom: first\r\n\
            Custom: second\r\n\r\n";

        let req = parse_request(input).unwrap();

        // The last value should be used for duplicate headers
        assert_eq!(req.headers.get("custom"), Some(&"second".to_string()));
        assert_eq!(
            req.headers.get("host"),
            Some(&"first.example.com".to_string())
        );
    }

    #[test]
    fn test_http_request_methods() {
        // Create a request with headers
        let mut headers = HashMap::new();
        headers.insert("host".to_string(), "example.com".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());

        let req = HttpRequest::new(
            Method::POST,
            "/api/data".to_string(),
            HttpVersion::Http11,
            headers,
        );

        // Test the get_header method (case-insensitive)
        assert_eq!(req.get_header("Host"), Some(&"example.com".to_string()));
        assert_eq!(req.get_header("host"), Some(&"example.com".to_string()));
        assert_eq!(req.get_header("HOST"), Some(&"example.com".to_string()));
        assert_eq!(
            req.get_header("Content-Type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(req.get_header("nonexistent"), None);

        // Test the has_header method (case-insensitive)
        assert!(req.has_header("Host"));
        assert!(req.has_header("host"));
        assert!(req.has_header("HOST"));
        assert!(req.has_header("Content-Type"));
        assert!(!req.has_header("nonexistent"));
    }

    #[test]
    fn test_complex_request() {
        // Complex HTTP request example with multiple headers
        let input = b"GET /docs/tutorials/linux/shellscripts/howto.html HTTP/1.1\r\n\
            Host: Linode.com\r\n\
            User-Agent: Mozilla/5.0 (Windows; U; Windows NT 6.1; en-US; rv:1.9.1.8) Gecko/20091102 Firefox/3.5.5\r\n\
            Accept: text/html,application/xhtml+xml,\r\n\
            Accept-Language: en-us\r\n\
            Accept-Encoding: gzip,deflate\r\n\
            Accept-Charset: ISO-8859-1,utf-8\r\n\
            Cache-Control: no-cache\r\n\r\n";

        let req = parse_request(input).unwrap();

        // Verify method, path, and version
        assert_eq!(req.method, Method::GET);
        assert_eq!(req.path, "/docs/tutorials/linux/shellscripts/howto.html");
        assert_eq!(req.version, HttpVersion::Http11);

        // Verify headers
        assert_eq!(req.headers.get("host"), Some(&"Linode.com".to_string()));
        assert_eq!(
            req.headers.get("user-agent"), 
            Some(&"Mozilla/5.0 (Windows; U; Windows NT 6.1; en-US; rv:1.9.1.8) Gecko/20091102 Firefox/3.5.5".to_string())
        );
        assert_eq!(
            req.headers.get("accept"),
            Some(&"text/html,application/xhtml+xml,".to_string())
        );
        assert_eq!(
            req.headers.get("accept-language"),
            Some(&"en-us".to_string())
        );
        assert_eq!(
            req.headers.get("accept-encoding"),
            Some(&"gzip,deflate".to_string())
        );
        assert_eq!(
            req.headers.get("accept-charset"),
            Some(&"ISO-8859-1,utf-8".to_string())
        );
        assert_eq!(
            req.headers.get("cache-control"),
            Some(&"no-cache".to_string())
        );

        // Test case-insensitive header access using the get_header method
        assert_eq!(
            req.get_header("User-Agent"), 
            Some(&"Mozilla/5.0 (Windows; U; Windows NT 6.1; en-US; rv:1.9.1.8) Gecko/20091102 Firefox/3.5.5".to_string())
        );
        assert_eq!(
            req.get_header("ACCEPT-LANGUAGE"),
            Some(&"en-us".to_string())
        );

        // Verify header existence using has_header method
        assert!(req.has_header("Host"));
        assert!(req.has_header("user-agent"));
        assert!(req.has_header("ACCEPT"));
        assert!(!req.has_header("nonexistent-header"));
    }
}

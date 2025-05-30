//! Tests for the HTTP parser.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde::{Deserialize, Serialize};

    use crate::parser::{HttpRequest, Method, HttpVersion, Error, parse_request};

    #[test]
    fn test_parse_simple_get_request() {
        let request = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.method, Method::GET);
        assert_eq!(result.path, "/index.html");
        assert_eq!(result.version, HttpVersion::Http11);
        assert_eq!(result.headers.get("Host").unwrap(), "example.com");
    }

    #[test]
    fn test_parse_request_with_multiple_headers() {
        let request = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nUser-Agent: test\r\nAccept: */*\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.method, Method::GET);
        assert_eq!(result.path, "/index.html");
        assert_eq!(result.version, HttpVersion::Http11);
        assert_eq!(result.headers.get("Host").unwrap(), "example.com");
        assert_eq!(result.headers.get("User-Agent").unwrap(), "test");
        assert_eq!(result.headers.get("Accept").unwrap(), "*/*");
    }

    #[test]
    fn test_case_insensitive_headers() {
        let request = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert!(result.has_header("host"));
        assert!(result.has_header("HOST"));
        assert!(result.has_header("Host"));
    }

    #[test]
    fn test_missing_host_header() {
        let request = b"GET /index.html HTTP/1.1\r\n\r\n";
        let result = parse_request(request);
        assert!(matches!(result, Err(Error::MissingHeader(ref h)) if h == "Host"));
    }

    #[test]
    fn test_invalid_method() {
        let request = b"INVALID /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request);
        assert!(matches!(result, Err(Error::InvalidMethod(ref m)) if m == "INVALID"));
    }

    #[test]
    fn test_invalid_http_version() {
        let request = b"GET /index.html HTTP/9.9\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request);
        assert!(matches!(result, Err(Error::InvalidVersion(ref v)) if v == "HTTP/9.9"));
    }

    #[test]
    fn test_invalid_header_format() {
        let request = b"GET /index.html HTTP/1.1\r\nInvalidHeader\r\n\r\n";
        let result = parse_request(request);
        assert!(matches!(result, Err(Error::InvalidHeaderFormat)));
    }

    #[test]
    fn test_empty_request() {
        let request = b"";
        let result = parse_request(request);
        assert!(matches!(result, Err(Error::EmptyRequest)));
    }

    #[test]
    fn test_incomplete_request_line() {
        let request = b"GET\r\n";
        let result = parse_request(request);
        assert!(matches!(result, Err(Error::MalformedRequestLine(_))));
    }

    #[test]
    fn test_http2_version() {
        let request = b"GET /index.html HTTP/2\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.method, Method::GET);
        assert_eq!(result.path, "/index.html");
        assert_eq!(result.version, HttpVersion::Http20);
        assert_eq!(result.headers.get("Host").unwrap(), "example.com");
    }

    #[test]
    fn test_all_methods() {
        let methods = vec![
            (b"GET /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec(), Method::GET),
            (b"POST /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec(), Method::POST),
            (b"PUT /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec(), Method::PUT),
            (b"DELETE /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec(), Method::DELETE),
            (b"HEAD /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec(), Method::HEAD),
            (b"OPTIONS /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec(), Method::OPTIONS),
            (b"PATCH /index.html HTTP/1.1\r\nHost: example.com\r\n\r\n".to_vec(), Method::PATCH),
        ];

        for (request, expected_method) in methods {
            let result = parse_request(&request).unwrap();
            assert_eq!(result.method, expected_method);
        }
    }

    #[test]
    fn test_headers_with_multiple_colons() {
        let request = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nX-Test: value:with:colons\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.headers.get("X-Test").unwrap(), "value:with:colons");
    }

    #[test]
    fn test_http10_version() {
        let request = b"GET /index.html HTTP/1.0\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.version, HttpVersion::Http10);
    }

    #[test]
    fn test_http10_without_host() {
        // HTTP/1.0 doesn't require a Host header
        let request = b"GET /index.html HTTP/1.0\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.method, Method::GET);
        assert_eq!(result.path, "/index.html");
        assert_eq!(result.version, HttpVersion::Http10);
        assert!(result.headers.is_empty());
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
        let request = b"GET /index.html HTTP/1.1\r\nHost: example.com  \r\nUser-Agent:  test  \r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.headers.get("Host").unwrap(), "example.com");
        assert_eq!(result.headers.get("User-Agent").unwrap(), "test");
    }

    #[test]
    fn test_mixed_line_endings() {
        let request = b"GET /index.html HTTP/1.1\r\nHost: example.com\nUser-Agent: test\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.method, Method::GET);
        assert_eq!(result.path, "/index.html");
        assert_eq!(result.version, HttpVersion::Http11);
        assert_eq!(result.headers.get("Host").unwrap(), "example.com");
        assert_eq!(result.headers.get("User-Agent").unwrap(), "test");
    }

    #[test]
    fn test_request_line_with_extra_whitespace() {
        let request = b"GET  /index.html  HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.method, Method::GET);
        assert_eq!(result.path, "/index.html");
        assert_eq!(result.version, HttpVersion::Http11);
    }

    #[test]
    fn test_empty_path() {
        let request = b"GET  HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request);
        assert!(matches!(result, Err(Error::MalformedRequestLine(_))));
    }

    #[test]
    fn test_path_with_query_parameters() {
        let request = b"GET /search?q=test&page=1 HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.path, "/search?q=test&page=1");
        assert_eq!(result.query_params.get("q").unwrap(), "test");
        assert_eq!(result.query_params.get("page").unwrap(), "1");
    }

    #[test]
    fn test_complex_query_parameters() {
        let request = b"GET /search?q=test%20query&filter=name:john&sort=date&page=1 HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.path, "/search?q=test%20query&filter=name:john&sort=date&page=1");
        assert_eq!(result.query_params.get("q").unwrap(), "test%20query");
        assert_eq!(result.query_params.get("filter").unwrap(), "name:john");
        assert_eq!(result.query_params.get("sort").unwrap(), "date");
        assert_eq!(result.query_params.get("page").unwrap(), "1");
    }

    #[test]
    fn test_query_parameters_without_values() {
        let request = b"GET /search?q=test&flag&empty= HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.path, "/search?q=test&flag&empty=");
        assert_eq!(result.query_params.get("q").unwrap(), "test");
        assert_eq!(result.query_params.get("flag").unwrap(), "");
        assert_eq!(result.query_params.get("empty").unwrap(), "");
    }

    #[test]
    fn test_malformed_utf8_in_request() {
        let request = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nX-Test: \xFF\xFF\xFF\r\n\r\n";
        let result = parse_request(request);
        assert!(matches!(result, Err(Error::MalformedRequestLine(ref s)) if s == "Invalid UTF-8"));
    }

    #[test]
    fn test_empty_header_value() {
        let request = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nX-Empty:\r\n\r\n";
        let result = parse_request(request).unwrap();
        assert_eq!(result.headers.get("X-Empty").unwrap(), "");
    }

    #[test]
    fn test_duplicate_headers() {
        let request = b"GET /index.html HTTP/1.1\r\nHost: example.com\r\nX-Test: value1\r\nX-Test: value2\r\n\r\n";
        let result = parse_request(request).unwrap();
        // The second value should overwrite the first
        assert_eq!(result.headers.get("X-Test").unwrap(), "value2");
    }

    #[test]
    fn test_http_request_methods() {
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "example.com".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let request = HttpRequest::new(Method::GET, "/index.html".to_string(), HttpVersion::Http11, headers.clone());

        // Test get_header
        assert_eq!(request.get_header("Host").unwrap(), "example.com");
        assert_eq!(request.get_header("host").unwrap(), "example.com");
        assert_eq!(request.get_header("HOST").unwrap(), "example.com");
        assert!(request.get_header("X-Test").is_none());

        // Test has_header
        assert!(request.has_header("Host"));
        assert!(request.has_header("host"));
        assert!(request.has_header("HOST"));
        assert!(!request.has_header("X-Test"));

        // Test is_json
        assert!(request.is_json());

        // Test with_body
        let body = b"{\"key\":\"value\"}".to_vec();
        let request_with_body = HttpRequest::with_body(Method::POST, "/api".to_string(), HttpVersion::Http11, headers, body);
        assert_eq!(request_with_body.method, Method::POST);
        assert_eq!(request_with_body.path, "/api");
        assert_eq!(request_with_body.version, HttpVersion::Http11);
        assert_eq!(request_with_body.body, b"{\"key\":\"value\"}".to_vec());
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestUser {
        name: String,
        email: String,
    }

    #[test]
    fn test_json_parsing() {
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "example.com".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let body = r#"{"name":"John Doe","email":"john@example.com"}"#.as_bytes().to_vec();
        let request = HttpRequest::with_body(Method::POST, "/api/users".to_string(), HttpVersion::Http11, headers.clone(), body.clone());

        // Test json parsing
        let user: TestUser = request.json().unwrap();
        assert_eq!(user.name, "John Doe");
        assert_eq!(user.email, "john@example.com");

        // Test json parsing with invalid content type
        let mut headers_no_json = headers.clone();
        headers_no_json.insert("Content-Type".to_string(), "text/plain".to_string());
        let request_no_json = HttpRequest::with_body(Method::POST, "/api/users".to_string(), HttpVersion::Http11, headers_no_json, body.clone());
        let result: Result<TestUser, _> = request_no_json.json();
        assert!(matches!(result, Err(Error::MissingHeader(_))));

        // Test json parsing with invalid JSON
        let invalid_body = r#"{"name":"John Doe","email":}"#.as_bytes().to_vec();
        let request_invalid_json = HttpRequest::with_body(Method::POST, "/api/users".to_string(), HttpVersion::Http11, headers, invalid_body);
        let result: Result<TestUser, _> = request_invalid_json.json();
        assert!(matches!(result, Err(Error::JsonError(_))));
    }

    #[test]
    fn test_complex_request() {
        let request = b"POST /api/users?role=admin HTTP/1.1\r\n\
            Host: example.com\r\n\
            User-Agent: test-client/1.0\r\n\
            Content-Type: application/json\r\n\
            Content-Length: 43\r\n\
            X-API-Key: secret-key\r\n\
            \r\n\
            {\"name\":\"John Doe\",\"email\":\"john@example.com\"}";

        let result = parse_request(request).unwrap();
        assert_eq!(result.method, Method::POST);
        assert_eq!(result.path, "/api/users?role=admin");
        assert_eq!(result.version, HttpVersion::Http11);
        assert_eq!(result.headers.get("Host").unwrap(), "example.com");
        assert_eq!(result.headers.get("User-Agent").unwrap(), "test-client/1.0");
        assert_eq!(result.headers.get("Content-Type").unwrap(), "application/json");
        assert_eq!(result.headers.get("Content-Length").unwrap(), "43");
        assert_eq!(result.headers.get("X-API-Key").unwrap(), "secret-key");
        assert_eq!(result.query_params.get("role").unwrap(), "admin");

        // Note: The body is not parsed in the current implementation of parse_request
        // This would require additional logic to read the body based on Content-Length
        // or Transfer-Encoding headers
    }
}

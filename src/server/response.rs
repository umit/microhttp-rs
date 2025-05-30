//! HTTP response types and utilities.

use std::collections::HashMap;
use serde::Serialize;

use crate::server::error::Error;

/// HTTP status codes with their standard reason phrases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusCode {
    /// 200 OK: The request has succeeded.
    Ok = 200,
    /// 201 Created: The request has been fulfilled and a new resource has been created.
    Created = 201,
    /// 202 Accepted: The request has been accepted for processing, but processing has not been completed.
    Accepted = 202,
    /// 204 No Content: The server has fulfilled the request but does not need to return a response body.
    NoContent = 204,
    /// 400 Bad Request: The server cannot process the request due to a client error.
    BadRequest = 400,
    /// 401 Unauthorized: Authentication is required and has failed or has not been provided.
    Unauthorized = 401,
    /// 403 Forbidden: The server understood the request but refuses to authorize it.
    Forbidden = 403,
    /// 404 Not Found: The requested resource could not be found.
    NotFound = 404,
    /// 405 Method Not Allowed: The request method is not supported for the requested resource.
    MethodNotAllowed = 405,
    /// 500 Internal Server Error: The server encountered an unexpected condition.
    InternalServerError = 500,
    /// 501 Not Implemented: The server does not support the functionality required to fulfill the request.
    NotImplemented = 501,
    /// 502 Bad Gateway: The server received an invalid response from an upstream server.
    BadGateway = 502,
    /// 503 Service Unavailable: The server is currently unable to handle the request.
    ServiceUnavailable = 503,
}

impl StatusCode {
    /// Get the reason phrase for this status code.
    pub fn reason_phrase(&self) -> &'static str {
        match self {
            StatusCode::Ok => "OK",
            StatusCode::Created => "Created",
            StatusCode::Accepted => "Accepted",
            StatusCode::NoContent => "No Content",
            StatusCode::BadRequest => "Bad Request",
            StatusCode::Unauthorized => "Unauthorized",
            StatusCode::Forbidden => "Forbidden",
            StatusCode::NotFound => "Not Found",
            StatusCode::MethodNotAllowed => "Method Not Allowed",
            StatusCode::InternalServerError => "Internal Server Error",
            StatusCode::NotImplemented => "Not Implemented",
            StatusCode::BadGateway => "Bad Gateway",
            StatusCode::ServiceUnavailable => "Service Unavailable",
        }
    }
}

/// Represents an HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// The HTTP status code
    pub status: StatusCode,
    /// The HTTP headers
    pub headers: HashMap<String, String>,
    /// The response body
    pub body: Vec<u8>,
}

impl HttpResponse {
    /// Create a new HTTP response with the given status code.
    pub fn new(status: StatusCode) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Server".to_string(), "microhttp-rs".to_string());

        Self {
            status,
            headers,
            body: Vec::new(),
        }
    }

    /// Set the response body with a string.
    pub fn with_body_string(mut self, body: impl Into<String>) -> Self {
        let body_string = body.into();
        self.body = body_string.into_bytes();
        let content_length = self.body.len().to_string();
        self.with_header("Content-Length", content_length)
    }

    /// Set the response body with bytes.
    pub fn with_body_bytes(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        let content_length = self.body.len().to_string();
        self.with_header("Content-Length", content_length)
    }

    /// Add or replace a header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Set the content type.
    pub fn with_content_type(self, content_type: impl Into<String>) -> Self {
        self.with_header("Content-Type", content_type)
    }

    /// Set the response body with a JSON value.
    ///
    /// This method serializes the provided value to JSON and sets it as the response body.
    pub fn with_json<T: Serialize>(self, value: &T) -> Result<Self, Error> {
        let json = serde_json::to_vec(value).map_err(Error::JsonError)?;
        Ok(self
            .with_header("Content-Type", "application/json")
            .with_body_bytes(json))
    }

    /// Convert the response to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Add the status line
        let status_line = format!("HTTP/1.1 {} {}\r\n", self.status as u16, self.status.reason_phrase());
        bytes.extend_from_slice(status_line.as_bytes());

        // Add the headers
        for (name, value) in &self.headers {
            let header_line = format!("{name}: {value}\r\n");
            bytes.extend_from_slice(header_line.as_bytes());
        }

        // Add the empty line that separates headers from body
        bytes.extend_from_slice(b"\r\n");

        // Add the body
        bytes.extend_from_slice(&self.body);

        bytes
    }
}

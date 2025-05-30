//! HTTP request methods.

use std::fmt;
use std::str::FromStr;

use crate::parser::error::Error;

/// HTTP request methods as defined in RFC 7231 and common extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// GET method: Requests a representation of the specified resource.
    GET,
    /// POST method: Submits data to be processed to the identified resource.
    POST,
    /// PUT method: Replaces all current representations of the target resource with the request payload.
    PUT,
    /// DELETE method: Deletes the specified resource.
    DELETE,
    /// HEAD method: Same as GET but only transfers the status line and header section.
    HEAD,
    /// OPTIONS method: Describes the communication options for the target resource.
    OPTIONS,
    /// PATCH method: Applies partial modifications to a resource.
    PATCH,
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
        write!(f, "{self:?}")
    }
}

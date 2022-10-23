pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub mod webbench;
pub mod protocol;

use clap::ValueEnum;

pub use self::webbench::{Config, Webbench};

/// Supported HTTP version
#[derive(Copy, Clone, Debug, ValueEnum, Eq, PartialEq)]
pub enum Version {
    /// HTTP 0.9
    H09,

    /// HTTP 1.0
    H10,

    /// HTTP 1.1
    H11,
}

impl Into<http::Version> for Version {
    fn into(self) -> http::Version {
        match self {
            Version::H09 => http::Version::HTTP_09,
            Version::H10 => http::Version::HTTP_10,
            Version::H11 => http::Version::HTTP_11,
        }
    }
}

/// Supported HTTP method
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Method {
    OPTIONS,
    GET,
    HEAD,
    TRACE,
}

impl Into<http::Method> for Method {
    fn into(self) -> http::Method {
        match self {
            Method::OPTIONS => http::Method::OPTIONS,
            Method::GET => http::Method::GET,
            Method::HEAD => http::Method::HEAD,
            Method::TRACE => http::Method::TRACE,
        }
    }
}

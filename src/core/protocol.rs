use http::{Request, Version};
use std::io::Write;

pub fn raw_request(request: Request<()>) -> super::Result<Vec<u8>> {
    let protocol = Http1::new(); // Only support HTTP 1
    protocol.raw_request(request)
}

fn version_to_str(version: Version) -> &'static str {
    match version {
        Version::HTTP_09 => "HTTP/0.9",
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        _ => "",
    }
}

trait Protocol {
    fn raw_request(&self, request: Request<()>) -> super::Result<Vec<u8>>;
}

struct Http1;

impl Http1 {
    fn new() -> Self {
        Self {}
    }
}

impl Protocol for Http1 {
    fn raw_request(&self, request: Request<()>) -> super::Result<Vec<u8>> {
        let mut raw = Vec::new();

        // Request line
        write!(raw, "{} {} {}\r\n", 
            request.method(), request.uri().path(), version_to_str(request.version()))?;

        // Headers
        for (name, value) in request.headers().iter() {
            write!(raw, "{}: {}\r\n", name, value.to_str()?)?;
        }

        write!(raw, "\r\n")?;

        Ok(raw)
    }
}

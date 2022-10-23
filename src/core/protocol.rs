use http::Request;
use std::io::Write;

pub fn raw_request(request: Request<()>) -> super::Result<Vec<u8>> {
    let protocol = Http::new();     // Only support HTTP
    protocol.raw_request(request)
}

trait Protocol {
    fn raw_request(&self, request: Request<()>) -> super::Result<Vec<u8>>;
}

struct Http;

impl Http {
    fn new() -> Self {
        Self {}
    }
}

impl Protocol for Http {
    fn raw_request(&self, request: Request<()>) -> super::Result<Vec<u8>> {
        let mut raw = Vec::new();

        // Request line
        write!(raw, "{} {} {:?}\r\n", 
            request.method(), request.uri().path(), request.version())?;

        // Headers
        for (name, value) in request.headers().iter() {
            write!(raw, "{}: {}\r\n", name, value.to_str()?)?;
        }

        write!(raw, "\r\n")?;

        Ok(raw)
    }
}

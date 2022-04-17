use hyper::{Request};
pub struct Webbench {
    request: Request<()>,

    client_count: usize,
    is_keepalive: bool,
    is_force: bool,
}

impl Webbench {
    pub fn new(request: Request<()>, client_count: usize, is_keepalive: bool, is_force: bool) -> Self {
        Self { request, client_count, is_keepalive, is_force }
    }

    pub fn start(&mut self) {  }
}

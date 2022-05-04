use hyper::Request;
use tokio::task;
use std::sync::atomic::{AtomicU32, AtomicU64};

#[derive(Debug)]
pub struct Config {
    pub request: Request<()>,
    pub is_keepalive: bool,
    pub is_force: bool,
    pub clients: usize,
}

#[derive(Debug, Default)]
pub struct Status {
    pub recived: AtomicU64,
    pub success: AtomicU32,
}

pub struct Webbench<'a> {
    config: &'a Config,
    status: Status,
}

impl<'a> Webbench<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config, status: Status::default() }
    }

    pub async fn start(&mut self) {

    }

    pub fn status(&self) -> &Status {
        &self.status
    }
}

use hyper::{Request, Client, Body, Method, Uri, Version, body::HttpBody};
use tokio::task::JoinHandle;
use std::{sync::{atomic::{AtomicU32, AtomicU64, Ordering, AtomicBool}, Arc}};

#[derive(Debug)]
pub struct Config {
    pub request_data: (Method, Uri, Version),
    pub is_keepalive: bool,
    pub is_force: bool,
    pub clients: usize,
}

impl Config {
    pub fn build_request(&self) -> Request<Body> {
        Request::builder()
            .method(self.request_data.0.clone())
            .uri(self.request_data.1.clone())
            .version(self.request_data.2)
            .header("Host", self.request_data.1.clone().to_string())
            .header("User-Agent", "webbench-rs")
            .header("Connection", if self.is_keepalive { "keep-alive" } else { "close" })
            .body(Body::empty())
            .unwrap()
    }
}

#[derive(Debug, Default)]
pub struct Status {
    pub recived: AtomicU64,
    pub success: AtomicU32,
    pub failed: AtomicU32,
    pub runnable: AtomicBool,
}

pub struct Webbench {
    config: Arc<Config>,
    status: Arc<Status>,
    join_list: Vec<JoinHandle<()>>,
}

impl Webbench {
    pub fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
            status: Arc::new(Status::default()),
            join_list: Vec::new()
        }
    }

    pub async fn start(&mut self) {
        self.status.runnable.store(true, Ordering::Relaxed);

        for _ in 0..self.config.clients {
            let config = self.config.clone();
            let status = self.status.clone();

            let f = tokio::spawn(async move {
                Self::benchmark(config, status).await;
            });

            self.join_list.push(f);
        }
    }

    pub async fn stop(&mut self) {
        self.status.runnable.store(false, Ordering::Release);

        for handle in self.join_list.iter_mut() {
            handle.abort();
        }
    }

    pub async fn wait(&mut self) {
        for join_handle in self.join_list.iter_mut() {
            let _ = join_handle.await;
        }
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    async fn benchmark(config: Arc<Config>, status: Arc<Status>) {
        let client = Client::builder()
            .set_host(false)
            .pool_max_idle_per_host(if config.is_keepalive { 1 } else { 0 })
            .build_http::<Body>();

        while status.runnable.load(Ordering::Acquire) {
            match client.request(config.build_request()).await {
                Ok(mut resp) => {
                    // Count body size
                    let mut recived = u64::default();
                    while let Some(chunk) = 
                        resp.body_mut().data().await.and_then(|chunk| {
                            if let Ok(chunk) = chunk { Some(chunk) } else { None }
                        }) {
                        recived += chunk.len() as u64;
                    }

                    status.recived.fetch_add(recived, Ordering::AcqRel);
                    status.success.fetch_add(1, Ordering::AcqRel);
                },
                Err(_) => {
                    status.failed.fetch_add(1, Ordering::AcqRel);
                }
            }
        }
    }
}

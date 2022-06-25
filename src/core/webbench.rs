use tokio::{runtime, net::TcpStream, io::{AsyncWriteExt, AsyncReadExt}};
use std::{sync::{atomic::{AtomicU32, AtomicU64, Ordering}, Arc}, time::Duration, net::SocketAddr};

#[derive(Debug)]
pub struct Config {
    pub addr: Vec<SocketAddr>,
    pub request: Vec<u8>,
    pub is_keepalive: bool,
    pub clients: usize,
}

#[derive(Debug, Default)]
pub struct Status {
    pub received: AtomicU64,
    pub success: AtomicU32,
    pub failed: AtomicU32,
}

struct Parts {
    config: Config,
    status: Status,
}

pub struct Webbench {
    inner: Arc<Parts>,
    runtime: runtime::Runtime,
}

impl Webbench {
    pub fn new(config: Config) -> super::Result<Self> {
        let runtime = runtime::Builder::new_multi_thread()
            .worker_threads(num_cpus::get_physical())
            .enable_io()
            .build()?;

        Ok(Self {
            inner: Arc::new(Parts {
                config,
                status: Status::default(),
            }),
            runtime,
        })
    }

    pub fn start(&self) {
        for _ in 0..self.inner.config.clients {
            if self.inner.config.is_keepalive {
                self.runtime.spawn(Self::bench_keepalive(self.inner.clone()));
            } else {
                self.runtime.spawn(Self::bench_close(self.inner.clone()));
            }
        }
    }

    pub fn stop(self) {
        self.runtime.shutdown_timeout(Duration::default());
    }

    pub fn status(&self) -> &Status {
        &self.inner.status
    }

    #[inline]
    async fn bench_keepalive(inner: Arc<Parts>) {
        let mut buf = [0; 1024];
        loop {
            let mut connection;
            match TcpStream::connect(&*inner.config.addr).await {
                Ok(c) => {
                    connection = c;
                    let _ = connection.set_nodelay(true);
                },
                Err(_) => {
                    inner.status.failed.fetch_add(1, Ordering::AcqRel);
                    continue;
                },
            };

            loop {
                if let Err(_) = connection.write_all(&inner.config.request).await {
                    inner.status.failed.fetch_add(1, Ordering::AcqRel);
                    break;
                }

                let mut acc = 0;
                while let Ok(n) = connection.read(&mut buf).await {
                    acc += n as u64;

                    if n < 1024 {
                        break;   // EOF
                    }
                }

                if acc == 0 {
                    inner.status.failed.fetch_add(1, Ordering::AcqRel);
                } else {
                    inner.status.received.fetch_add(acc, Ordering::AcqRel);
                    inner.status.success.fetch_add(1, Ordering::AcqRel);
                }
            }
        }
    }

    #[inline]
    async fn bench_close(inner: Arc<Parts>) {
        let mut buf = [0; 1024];

        loop {
            let mut connection;

            match TcpStream::connect(&*inner.config.addr).await {
                Ok(c) => {
                    connection = c;
                    let _ = connection.set_nodelay(true);
                },
                Err(_) => {
                    inner.status.failed.fetch_add(1, Ordering::AcqRel);
                    continue;
                },
            }

            if let Err(_) = connection.write_all(&inner.config.request).await {
                inner.status.failed.fetch_add(1, Ordering::AcqRel);
                continue;
            }

            let mut acc = 0;
            while let Ok(n) = connection.read(&mut buf).await {
                acc += n as u64;

                if n < 1024 {
                    break;   // EOF
                }
            }

            if acc == 0 {
                inner.status.failed.fetch_add(1, Ordering::AcqRel);
            } else {
                inner.status.received.fetch_add(acc, Ordering::AcqRel);
                inner.status.success.fetch_add(1, Ordering::AcqRel);
            }
        }
    }
}

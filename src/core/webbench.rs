use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    runtime,
};

#[derive(Debug)]
pub struct Config {
    pub addrs: Vec<SocketAddr>,
    pub request: Vec<u8>,
    pub is_keepalive: bool,
    pub clients: u32,
}

#[derive(Debug, Default)]
pub struct Status {
    pub interrupted: AtomicBool,
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

    pub fn start(&self) -> super::Result<()> {
        std::net::TcpStream::connect(&*self.inner.config.addrs)?;

        let inner = self.inner.clone();
        self.runtime.spawn(async move {
            _ = tokio::signal::ctrl_c().await.map_err(|e| println!("{}", e));

            inner.status.interrupted.store(true, Ordering::Release);
        });

        for _ in 0..self.inner.config.clients {
            match self.inner.config.is_keepalive {
                true => self
                    .runtime
                    .spawn(Self::bench_keepalive(self.inner.clone())),
                false => self.runtime.spawn(Self::bench_close(self.inner.clone())),
            };
        }

        Ok(())
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
            match TcpStream::connect(&*inner.config.addrs).await {
                Ok(c) => {
                    connection = c;
                    let _ = connection.set_nodelay(true);
                }
                Err(_) => {
                    inner.status.failed.fetch_add(1, Ordering::AcqRel);
                    continue;
                }
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
                        break; // EOF
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

            match TcpStream::connect(&*inner.config.addrs).await {
                Ok(c) => {
                    connection = c;
                    let _ = connection.set_nodelay(true);
                }
                Err(_) => {
                    inner.status.failed.fetch_add(1, Ordering::AcqRel);
                    continue;
                }
            }

            if let Err(_) = connection.write_all(&inner.config.request).await {
                inner.status.failed.fetch_add(1, Ordering::AcqRel);
                continue;
            }

            let mut acc = 0;
            while let Ok(n) = connection.read(&mut buf).await {
                acc += n as u64;

                if n < 1024 {
                    break; // EOF
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

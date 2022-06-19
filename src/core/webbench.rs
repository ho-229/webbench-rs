use tokio::{runtime, net::TcpStream, io::AsyncWriteExt};
use std::{sync::{atomic::{AtomicU32, AtomicU64, Ordering}, Arc}, time::Duration};

#[derive(Debug)]
pub struct Config {
    pub addr: (String, u16),
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
            self.runtime.spawn(Self::benchmark(self.inner.clone()));
        }
    }

    pub fn stop(self) {
        self.runtime.shutdown_timeout(Duration::default());
    }

    pub fn status(&self) -> &Status {
        &self.inner.status
    }

    async fn benchmark(inner: Arc<Parts>) {
        let mut buf = [0; 1024];
        
        if inner.config.is_keepalive {
            loop {
                let mut connection;
                match TcpStream::connect(&inner.config.addr).await {
                    Ok(c) => {
                        connection = c;
                        let _ = connection.set_nodelay(true);
                    },
                    Err(_) => {
                        inner.status.failed.fetch_add(1, Ordering::AcqRel);
                        continue;
                    },
                };

                'keep: loop {
                    let mut ret = 0;
                    while match connection.write(&inner.config.request[ret..]).await {
                        Ok(n) if n < inner.config.request.len() => { ret = n; true }
                        Ok(_) => false,
                        Err(_) => {
                            inner.status.failed.fetch_add(1, Ordering::AcqRel);
                            break 'keep;
                        },
                    } {}

                    if let Err(_) = connection.readable().await {
                        break;   // Re-connect
                    }

                    while let Ok(n) = connection.try_read(&mut buf) {
                        if n == 0 {
                            break;   // EOF
                        }

                        inner.status.received.fetch_add(n as u64, Ordering::AcqRel);
                    }

                    inner.status.success.fetch_add(1, Ordering::AcqRel);
                }
            }
        } else {
            'close: loop {
                let mut ret = 0;
                let mut connection;

                match TcpStream::connect(&inner.config.addr).await {
                    Ok(c) => {
                        connection = c;
                        let _ = connection.set_nodelay(true);
                    },
                    Err(_) => {
                        inner.status.failed.fetch_add(1, Ordering::AcqRel);
                        continue;
                    },
                }

                while match connection.write(&inner.config.request[ret..]).await {
                    Ok(n) if n < inner.config.request.len() => { ret = n; true }
                    Ok(_) => false,
                    Err(_) => {
                        inner.status.failed.fetch_add(1, Ordering::AcqRel);
                        continue 'close;
                    },
                } {}

                if let Err(_) = connection.readable().await {
                    inner.status.failed.fetch_add(1, Ordering::AcqRel);
                    continue;   // Re-connect
                }

                while let Ok(n) = connection.try_read(&mut buf) {
                    if n == 0{
                        break;
                    }

                    inner.status.received.fetch_add(n as u64, Ordering::AcqRel);
                }

                inner.status.success.fetch_add(1, Ordering::AcqRel);
            }
        }
    }
}

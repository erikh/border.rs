#![allow(dead_code)]
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use crate::{config::Config, record_type::RecordType};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LBKind {
    #[serde(rename = "tcp", alias = "TCP")]
    TCP,
    #[serde(rename = "http", alias = "HTTP")]
    HTTP,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TLSSettings {
    certificate: String,
    key: String,
}

pub struct LB {
    config: Config,
    record: RecordType,
}

struct StreamContainer(TcpStream);

type BackendCount = BTreeMap<SocketAddr, AtomicU64>;

impl LB {
    pub fn new(config: Config, record: RecordType) -> Result<Self, anyhow::Error> {
        match record {
            RecordType::LB { .. } => {}
            _ => return Err(anyhow!("Record type was not LB")),
        }

        Ok(Self { config, record })
    }

    fn listen_addrs(&self) -> Result<Option<Vec<SocketAddr>>, anyhow::Error> {
        match &self.record {
            RecordType::LB { listeners, .. } => {
                let mut addresses: Option<Vec<SocketAddr>> = None;

                for listener in listeners {
                    if listener.name() == self.config.me {
                        addresses = listener.addr(self.config.clone());
                        break;
                    }
                }

                Ok(addresses)
            }
            _ => Err(anyhow!("Record type was not LB")),
        }
    }

    fn backends(&self) -> Result<Vec<SocketAddr>, anyhow::Error> {
        match &self.record {
            RecordType::LB { backends, .. } => Ok(backends.clone()),
            _ => Err(anyhow!("Record type was not LB")),
        }
    }

    fn kind(&self) -> Result<LBKind, anyhow::Error> {
        match &self.record {
            RecordType::LB { kind, .. } => Ok(kind.clone()),
            _ => Err(anyhow!("Record type was not LB")),
        }
    }

    async fn serve_tcp(&self, context: Arc<AtomicBool>) -> Result<(), anyhow::Error> {
        let addresses = self.listen_addrs()?.expect("No addresses to bind to");

        for address in addresses {
            tokio::spawn(Self::serve_tcp_listener(
                context.clone(),
                self.config.clone(),
                self.backends()?,
                address,
            ));
        }

        Ok(())
    }

    async fn serve_tcp_listener(
        context: Arc<AtomicBool>,
        _config: Config,
        mut backends: Vec<SocketAddr>,
        address: SocketAddr,
    ) -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind(address).await?;
        let backend_count = Arc::new(Mutex::new(BackendCount::default()));

        loop {
            if context.load(Ordering::Relaxed) {
                return Ok(());
            }

            let socket = listener.accept().await?;

            'retry: loop {
                let mut lowest_backend: Option<SocketAddr> = None;
                let lowest_backend_count = AtomicU64::default();

                for backend in &backends {
                    match backend_count.lock().await.get(backend) {
                        Some(count) => {
                            if count.load(Ordering::Relaxed)
                                <= lowest_backend_count.load(Ordering::Relaxed)
                            {
                                lowest_backend = Some(*backend);
                                lowest_backend_count
                                    .store(count.load(Ordering::Acquire), Ordering::SeqCst)
                            }
                        }
                        None => {
                            lowest_backend = Some(*backend);
                            lowest_backend_count.store(0, Ordering::SeqCst);
                        }
                    }
                }

                if let None = lowest_backend {
                    lowest_backend = Some(
                        *backends
                            .iter()
                            .nth(0)
                            .expect("Could not find any backends to service"),
                    );

                    lowest_backend_count.store(
                        backend_count
                            .lock()
                            .await
                            .get(&lowest_backend.unwrap())
                            .unwrap()
                            .load(Ordering::Acquire),
                        Ordering::SeqCst,
                    );
                }

                let backend = lowest_backend.unwrap();

                if backend_count.lock().await.contains_key(&backend) {
                    backend_count
                        .lock()
                        .await
                        .get(&backend)
                        .unwrap()
                        .fetch_add(1, Ordering::Acquire);
                } else {
                    backend_count
                        .lock()
                        .await
                        .insert(backend, lowest_backend_count);
                }

                match TcpStream::connect(backend).await {
                    Ok(stream) => {
                        let socket = Arc::new(Mutex::new(Box::new(socket)));
                        let stream = Arc::new(Mutex::new(Box::new(StreamContainer(stream))));
                        let backend_count = backend_count.clone();

                        tokio::spawn(async move {
                            match tokio::io::copy_bidirectional(
                                &mut socket.lock().await.0,
                                &mut stream.lock().await.0,
                            )
                            .await
                            {
                                Ok(_) => {}
                                Err(_) => {}
                            }

                            backend_count
                                .lock()
                                .await
                                .get(&backend)
                                .unwrap()
                                .fetch_sub(1, Ordering::Acquire);
                        });
                    }
                    // FIXME logging
                    Err(e) => {
                        eprintln!("{}", e);
                        backends.retain(|be| *be != backend);
                        continue 'retry;
                    }
                };

                break;
            }
        }
    }

    pub async fn serve(&self, context: Arc<AtomicBool>) -> Result<(), anyhow::Error> {
        match self.kind() {
            Ok(LBKind::TCP) => self.serve_tcp(context).await,
            Ok(LBKind::HTTP) => Err(anyhow!("HTTP load balancers aren't supported yet")),
            Err(e) => Err(e),
        }
    }
}

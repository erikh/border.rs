#![allow(dead_code)]
use std::{collections::BTreeMap, net::SocketAddr, sync::Arc};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, RwLock},
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

type BackendCount = BTreeMap<SocketAddr, u64>;

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

    async fn serve_tcp(&self) -> Result<(), anyhow::Error> {
        let addresses = self.listen_addrs()?.expect("No addresses to bind to");

        for address in addresses {
            tokio::spawn(Self::serve_tcp_listener(
                self.config.clone(),
                self.backends()?,
                address,
            ));
        }

        Ok(())
    }

    async fn serve_tcp_listener(
        // FIXME some kind of context to cancel this routine
        _config: Config,
        mut backends: Vec<SocketAddr>,
        address: SocketAddr,
    ) -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind(address).await?;
        let backend_count = Arc::new(RwLock::new(BackendCount::default()));

        loop {
            let socket = listener.accept().await?;

            'retry: loop {
                let mut lowest_backend: Option<SocketAddr> = None;
                let mut lowest_backend_count: u64 = 0;

                for backend in &backends {
                    match backend_count.read().await.get(backend) {
                        Some(count) => {
                            if lowest_backend_count <= *count {
                                lowest_backend = Some(*backend);
                                lowest_backend_count = *count;
                            }
                        }
                        None => {
                            lowest_backend = Some(*backend);
                            lowest_backend_count = 0;
                        }
                    }
                }

                if let None = lowest_backend {
                    // FIXME what do? is this even possible?
                    lowest_backend = Some(
                        *backends
                            .iter()
                            .nth(0)
                            .expect("Could not find any backends to service"),
                    );
                    lowest_backend_count = *backend_count
                        .read()
                        .await
                        .get(&lowest_backend.unwrap())
                        .unwrap_or(&0);
                }

                let backend = lowest_backend.unwrap();

                lowest_backend_count += 1;
                backend_count
                    .write()
                    .await
                    .insert(backend, lowest_backend_count);

                match TcpStream::connect(backend).await {
                    Ok(stream) => {
                        let socket = Arc::new(Mutex::new(Box::new(socket)));
                        let stream = Arc::new(Mutex::new(Box::new(StreamContainer(stream))));
                        let backend_count = backend_count.clone();

                        tokio::spawn(async move {
                            tokio::io::copy_bidirectional(
                                &mut socket.lock().await.0,
                                &mut stream.lock().await.0,
                            )
                            .await
                            .unwrap();

                            backend_count.write().await.insert(
                                backend,
                                backend_count.read().await.get(&backend).unwrap() - 1,
                            );
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

    pub async fn serve(&self) -> Result<(), anyhow::Error> {
        match self.kind() {
            Ok(LBKind::TCP) => self.serve_tcp().await,
            Ok(LBKind::HTTP) => Err(anyhow!("HTTP load balancers aren't supported yet")),
            Err(e) => Err(e),
        }
    }
}

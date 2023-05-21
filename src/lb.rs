#![allow(dead_code)]
use std::{net::SocketAddr, sync::Arc};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

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
            let obj = self.clone();
            tokio::spawn(obj.serve_tcp_listener(TcpListener::bind(address).await?));
        }

        Ok(())
    }

    async fn serve_tcp_listener(self, listener: TcpListener) -> Result<(), anyhow::Error> {
        let backends = self.backends();

        Ok(())
    }

    pub async fn serve(&self) -> Result<(), anyhow::Error> {
        match self.kind() {
            Ok(LBKind::TCP) => self.serve_tcp().await,
            Ok(LBKind::HTTP) => Err(anyhow!("HTTP load balancers aren't supported yet")),
            Err(e) => Err(e),
        }
    }
}

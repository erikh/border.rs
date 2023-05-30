#![allow(dead_code)]
use crate::{config::SafeConfig, dns_name::DNSName, listener::Listener};
use anyhow::anyhow;
use fancy_duration::FancyDuration;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum HealthCheckType {
    #[default]
    #[serde(rename = "tcp", alias = "TCP")]
    TCP,
}

#[derive(Debug, Clone)]
pub enum HealthCheckTargetType {
    DNS,
    LBBackend,
    LBFrontend,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    failures: u8,
    timeout: FancyDuration<Duration>,
    #[serde(rename = "type", default)]
    typ: HealthCheckType,
}

#[derive(Clone)]
pub struct HealthCheckAction {
    healthcheck: HealthCheck,
    target: SocketAddr,
    target_type: HealthCheckTargetType,
    target_name: DNSName,
    listener: Option<Listener>,
}

#[derive(Clone)]
pub struct HealthChecker {
    actions: Vec<HealthCheckAction>,
    config: SafeConfig,
}

impl HealthCheck {
    pub fn to_action(
        self,
        target: SocketAddr,
        target_type: HealthCheckTargetType,
        target_name: DNSName,
        listener: Option<Listener>,
    ) -> HealthCheckAction {
        HealthCheckAction {
            healthcheck: self,
            target,
            target_type,
            target_name,
            listener,
        }
    }
}

impl HealthCheckAction {
    async fn check(&self) -> Result<(), anyhow::Error> {
        match self.healthcheck.typ {
            HealthCheckType::TCP => self.check_tcp().await,
        }
    }

    async fn check_tcp(&self) -> Result<(), anyhow::Error> {
        match TcpStream::connect(self.target).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn add_config(&self, config: SafeConfig) {
        match self.target_type {
            HealthCheckTargetType::DNS => {
                let mut config = config.lock().await;

                for (_, zone) in &mut config.zones {
                    for record in &mut zone.records {
                        if record.name == self.target_name {
                            if let Some(lis) = &self.listener {
                                record.add_listener(lis.clone());
                            }

                            record.add_ip(self.target.ip())
                        }
                    }
                }
            }
            HealthCheckTargetType::LBFrontend => {
                let mut config = config.lock().await;

                for (_, zone) in &mut config.zones {
                    for record in &mut zone.records {
                        if record.name == self.target_name {
                            if let Some(lis) = &self.listener {
                                record.add_listener(lis.clone());
                            }
                        }
                    }
                }
            }
            HealthCheckTargetType::LBBackend => {
                let mut config = config.lock().await;

                for (_, zone) in &mut config.zones {
                    for record in &mut zone.records {
                        if record.name == self.target_name {
                            record.add_backend(self.target);
                        }
                    }
                }
            }
        }

        // FIXME trigger DNS reload context
    }

    async fn remove_config(&mut self, config: SafeConfig) {
        match self.target_type {
            HealthCheckTargetType::DNS => {
                let mut config = config.lock().await;
                for (_, zone) in &mut config.zones {
                    for record in &mut zone.records {
                        if record.name == self.target_name {
                            record.remove_ip(self.target.ip());
                        }
                    }
                }
            }
            HealthCheckTargetType::LBBackend => {
                let mut config = config.lock().await;
                for (_, zone) in &mut config.zones {
                    for record in &mut zone.records {
                        if record.name == self.target_name {
                            record.remove_backend(self.target);
                        }
                    }
                }
            }
            HealthCheckTargetType::LBFrontend => {
                let mut config = config.lock().await;
                for (_, zone) in &mut config.zones {
                    for record in &mut zone.records {
                        if record.name == self.target_name {
                            if let Some(lis) = &self.listener {
                                record.remove_listener(lis.clone())
                            }

                            record.remove_ip(self.target.ip());
                        }
                    }
                }
            }
        }

        // FIXME trigger DNS reload context
    }
}

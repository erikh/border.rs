#![allow(dead_code)]
use anyhow::anyhow;
use fancy_duration::FancyDuration;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpStream;

use crate::{config::SafeConfig, dns_name::DNSName, record_type::RecordType};

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
    target_name: Option<DNSName>,
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
        target_name: Option<DNSName>,
    ) -> HealthCheckAction {
        HealthCheckAction {
            healthcheck: self,
            target,
            target_type,
            target_name,
        }
    }
}

impl HealthCheckAction {
    // chi chiggity check yo self
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

    async fn adjust_config(&self, config: SafeConfig) {
        match self.target_type {
            HealthCheckTargetType::DNS => {
                let target_name = self.target_name.clone().unwrap();

                let mut zones = config.lock().await.zones.clone();

                for zone in &mut zones {
                    for record in &mut zone.1.records {
                        if record.name == target_name {
                            match &mut record.record {
                                RecordType::A { addresses, .. } => {
                                    addresses.retain(|addr| *addr != self.target.ip())
                                }
                                RecordType::LB { listeners, .. } => {
                                    let mut newlis = Vec::new();
                                    for lis in &mut *listeners {
                                        if lis
                                            .addr(config.clone())
                                            .await
                                            .unwrap()
                                            .contains(&self.target)
                                        {
                                            newlis.push(lis.clone());
                                        }
                                    }

                                    listeners.clear();
                                    listeners.append(&mut newlis);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                config.lock().await.zones = zones;
            }
            HealthCheckTargetType::LBBackend => {}
            HealthCheckTargetType::LBFrontend => {}
        }
    }
}

#![allow(dead_code)]
use crate::{config::SafeConfig, dns_name::DNSName, listener::Listener};
use anyhow::anyhow;
use fancy_duration::FancyDuration;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    time::{Duration, SystemTime},
};
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
    failure_count: u8,
    last_failure: Option<SystemTime>,
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
            failure_count: 0,
            last_failure: None,
        }
    }
}

impl HealthChecker {
    pub fn new(actions: Vec<HealthCheckAction>, config: SafeConfig) -> Self {
        Self { actions, config }
    }

    pub async fn run(&mut self) {
        loop {
            for check in &mut self.actions {
                check.perform(self.config.clone()).await;
            }

            tokio::time::sleep(Duration::new(1, 0)).await;
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
    }

    async fn remove_config(&self, config: SafeConfig) {
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
    }

    pub async fn perform(&mut self, config: SafeConfig) {
        match self.check().await {
            Ok(_) => {
                if self.failure_count >= self.healthcheck.failures {
                    self.add_config(config).await;
                }

                self.failure_count = 0;
                self.last_failure = None;
            }
            // FIXME log
            Err(_) => {
                self.failure_count += 1;
                self.last_failure = Some(SystemTime::now());

                if self.healthcheck.failures <= self.failure_count {
                    self.remove_config(config).await;
                }
            }
        }
    }
}

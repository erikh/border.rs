#![allow(dead_code)]
use fancy_duration::FancyDuration;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};

use crate::config::SafeConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    failures: u8,
    timeout: FancyDuration<Duration>,
}

#[derive(Clone)]
pub struct HealthCheckAction {
    healthcheck: HealthCheck,
    target: SocketAddr,
    config: SafeConfig,
}

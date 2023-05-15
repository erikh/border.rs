use fancy_duration::FancyDuration;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    failures: u8,
    timeout: FancyDuration<Duration>,
}

use crate::{
    health_check::HealthCheck,
    lb::{LBKind, TLSSettings},
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};

pub trait ToRecord {
    fn to_record(&self);
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RecordType {
    #[serde(rename = "a", alias = "A")]
    A {
        addresses: Vec<IpAddr>,
        #[serde(default = "default_ttl")]
        ttl: u32,
        healthcheck: Vec<HealthCheck>,
    },
    #[serde(rename = "txt", alias = "TXT")]
    TXT {
        value: Vec<String>,
        #[serde(default = "default_ttl")]
        ttl: u32,
    },
    #[serde(rename = "lb", alias = "LB")]
    LB {
        backends: Vec<SocketAddr>,
        kind: LBKind,
        listeners: Vec<String>,
        tls: Option<TLSSettings>,
        healthcheck: Vec<HealthCheck>,
    },
}

impl ToRecord for RecordType {
    fn to_record(&self) {}
}

fn default_ttl() -> u32 {
    30
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SOA {
    domain: String,
    admin: String,
    minttl: u32,
    serial: u32,
    refresh: u32,
    retry: u32,
    expire: u32,
}

impl ToRecord for SOA {
    fn to_record(&self) {}
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NS {
    servers: Vec<String>,
    #[serde(default = "default_ttl")]
    ttl: u32,
}

impl ToRecord for NS {
    fn to_record(&self) {}
}

#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum RecordType {
    A {
        addresses: Vec<IpAddr>,
        #[serde(skip_serializing_if = "Option::is_none", default = "default_ttl")]
        ttl: Option<u32>,
        healthcheck: HealthCheck,
    },
    TXT {
        value: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none", default = "default_ttl")]
        ttl: Option<u32>,
    },
    LB {
        backends: Vec<SocketAddr>,
        kind: LBKind,
        listeners: Vec<String>,
        tls: TLSSettings,
        healthcheck: HealthCheck,
    },
}

#[derive(Serialize, Deserialize)]
pub(crate) enum LBKind {
    TCP,
    HTTP,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TLSSettings {
    cert: String,
    key: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct HealthCheck {
    failures: u8,
    timeout: u16,
}

impl ToRecord for RecordType {
    fn to_record(&self) {}
}

fn default_ttl() -> Option<u32> {
    Some(30)
}

pub trait ToRecord {
    fn to_record(&self);
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct SOA {
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

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct NS {
    servers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default = "default_ttl")]
    ttl: Option<u32>,
}

impl ToRecord for NS {
    fn to_record(&self) {}
}

#![allow(dead_code)]
use crate::record_type::{RecordType, NS, SOA};
use josekit::jwk::Jwk;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use url::Url;

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    auth_key: Jwk,
    listen: ListenConfig,
    peers: Vec<Peer>,
    zones: Vec<Zone>,
    shutdown_wait: u8,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ListenConfig {
    dns: SocketAddr,
    control: SocketAddr,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Peer {
    ips: Vec<IpAddr>,
    control_server: Url,
    key: Jwk,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Zone {
    soa: SOA,
    ns: NS,
    records: Vec<Record>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Record {
    name: String,
    record: RecordType,
}

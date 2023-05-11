#![allow(dead_code)]
use crate::record_type::{RecordType, NS, SOA};
use josekit::jwk::Jwk;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    auth_key: Jwk,
    listen: ListenConfig,
    peers: Vec<Peer>,
    zones: BTreeMap<String, Zone>,
    shutdown_wait: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListenConfig {
    dns: SocketAddr,
    control: SocketAddr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Peer {
    ips: Vec<IpAddr>,
    control_server: Url,
    key: Jwk,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    soa: SOA,
    ns: NS,
    records: Vec<Record>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    name: String,
    record: RecordType,
}

use crate::{
    dns_name::DNSName,
    record_type::{RecordType, NS, SOA},
};
use josekit::jwk::Jwk;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub auth_key: Jwk,
    pub listen: ListenConfig,
    pub peers: Vec<Peer>,
    pub zones: BTreeMap<String, Zone>,
    pub shutdown_wait: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListenConfig {
    dns: SocketAddr,
    control: SocketAddr,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Peer {
    pub ips: Vec<IpAddr>,
    pub control_server: Url,
    pub key: Jwk,
}

impl Peer {
    pub fn name(&self) -> String {
        self.key
            .key_id()
            .expect("Expected the key id to be populated")
            .to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Zone {
    soa: SOA,
    ns: NS,
    records: Vec<Record>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    name: DNSName,
    record: RecordType,
}

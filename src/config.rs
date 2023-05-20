use crate::{
    dns_name::DNSName,
    record_type::{RecordType, NS, SOA},
};
use fancy_duration::FancyDuration;
use josekit::jwk::Jwk;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub auth_key: Jwk,
    pub listen: ListenConfig,
    pub peers: Vec<Peer>,
    pub zones: BTreeMap<DNSName, Zone>,
    #[serde(skip)]
    pub me: String,
    pub shutdown_wait: FancyDuration<Duration>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListenConfig {
    pub dns: SocketAddr,
    pub control: SocketAddr,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub soa: SOA,
    pub ns: NS,
    pub records: Vec<Record>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub name: DNSName,
    pub record: RecordType,
}

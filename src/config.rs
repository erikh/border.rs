#![allow(dead_code)]
use crate::record_type::{ToRecord, NS, SOA};
use josekit::jwk::Jwk;
use std::net::{IpAddr, SocketAddr};
use url::Url;

pub(crate) struct Config {
    auth_key: Jwk,
    listen: ListenConfig,
    peers: Vec<Peer>,
}

pub(crate) struct ListenConfig {
    dns: SocketAddr,
    control: SocketAddr,
}

pub(crate) struct Peer {
    ips: Vec<IpAddr>,
    control_server: Url,
    key: Jwk,
}

pub(crate) struct Zone {
    soa: SOA,
    ns: NS,
    records: Vec<Record<dyn ToRecord>>,
}

pub(crate) struct Record<T: ToRecord + ?Sized> {
    typ: String,
    name: String,
    value: Box<T>,
}

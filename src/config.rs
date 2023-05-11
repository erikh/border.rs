#![allow(dead_code)]
use crate::record_type::{RecordType, NS, SOA};
use josekit::jwk::Jwk;
use serde::{de::Visitor, Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    net::{IpAddr, SocketAddr},
};
use trust_dns_server::proto::rr::Name;
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
    name: DNSName,
    record: RecordType,
}

#[derive(Debug)]
pub struct DNSName(Name);

impl Serialize for DNSName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

struct DNSNameVisitor;

impl Visitor<'_> for DNSNameVisitor {
    type Value = DNSName;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expecting a DNS name")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(DNSName(match Name::parse(v, None) {
            Ok(res) => res,
            Err(e) => return Err(serde::de::Error::custom(e)),
        }))
    }
}

impl<'de> Deserialize<'de> for DNSName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(DNSNameVisitor)
    }
}

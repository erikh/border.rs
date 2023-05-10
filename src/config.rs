#![allow(dead_code)]
use crate::record_type::{RecordValue, NS, SOA};
use josekit::jwk::Jwk;
use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use url::Url;

#[derive(Serialize, Deserialize)]
pub(crate) struct Config {
    auth_key: Jwk,
    listen: ListenConfig,
    peers: Vec<Peer>,
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

pub(crate) struct Record {
    name: String,
    value: RecordValue,
}

impl Default for Record {
    fn default() -> Self {
        Self {
            name: String::default(),
            value: RecordValue::NULL,
        }
    }
}

impl Serialize for Record {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;

        map.serialize_entry("type", self.value.record_type());
        map.serialize_entry("name", &self.name);
        map.serialize_entry("value", &self.value.record_value());
        map.end()
    }
}

struct RecordVisitor;

impl<'de> Visitor<'de> for RecordVisitor {
    type Value = Record;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expecting a record entry")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut rec = Record::default();

        while let Some(key) = map.next_key()? {
            match key {
                "name" => rec.name = map.next_value()?,
                "value" => rec.value = map.next_value()?,
                _ => {
                    return Err(serde::de::Error::unknown_field(
                        key,
                        &["type", "name", "value"],
                    ))
                }
            }
        }

        Ok(rec)
    }
}

impl<'de> Deserialize<'de> for Record {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(RecordVisitor)
    }
}

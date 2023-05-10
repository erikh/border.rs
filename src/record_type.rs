#![allow(dead_code)]
use anyhow::anyhow;
use serde::{de::Visitor, Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Serialize)]
pub enum RecordValue {
    A(Box<A>),
    TXT(Box<TXT>),
    NULL,
}

impl RecordValue {
    pub fn record_type(&self) -> &str {
        match self {
            Self::A(_) => "a",
            Self::TXT(_) => "txt",
            Self::NULL => "null",
        }
    }

    pub fn record_value(self) -> Option<Box<dyn ToRecord>> {
        match self {
            Self::A(a) => Some(a),
            Self::TXT(txt) => Some(txt),
            Self::NULL => None,
        }
    }

    pub fn from_value(typ: &str, value: Box<dyn ToRecord>) -> Result<Self, anyhow::Error> {
        match typ {
            "a" | "A" => Ok(Self::A(value)),
            "txt" | "TXT" => Ok(Self::TXT(value)),
            _ => Err(anyhow!("invalid record type")),
        }
    }
}

impl Default for RecordValue {
    fn default() -> Self {
        Self::NULL
    }
}

struct RecordValueVisitor;

impl<'de> Visitor<'de> for RecordValueVisitor {
    type Value = RecordValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expected record information")
    }

    // fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    // where
    //     A: serde::de::MapAccess<'de>,
    // {
    // }
}

impl<'de> Deserialize<'de> for RecordValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(RecordValueVisitor)
    }
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

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct A {
    addresses: Vec<IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none", default = "default_ttl")]
    ttl: Option<u32>,
    // FIXME healthcheck
}

impl ToRecord for A {
    fn to_record(&self) {}
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct TXT {
    value: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default = "default_ttl")]
    ttl: Option<u32>,
}

impl ToRecord for TXT {
    fn to_record(&self) {}
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct NULL {}

impl ToRecord for NULL {
    fn to_record(&self) {}
}

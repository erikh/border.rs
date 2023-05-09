#![allow(dead_code)]
use std::net::IpAddr;

pub trait ToRecord {
    fn to_record(&self);
}

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

pub(crate) struct NS {
    servers: Vec<String>,
    ttl: u32,
}

impl ToRecord for NS {
    fn to_record(&self) {}
}

pub(crate) struct A {
    addresses: Vec<IpAddr>,
    ttl: u32,
    // FIXME healthcheck
}

impl ToRecord for A {
    fn to_record(&self) {}
}

pub(crate) struct TXT {
    value: Vec<String>,
    ttl: u32,
}

impl ToRecord for TXT {
    fn to_record(&self) {}
}

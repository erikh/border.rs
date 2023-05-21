use crate::{
    config::Config,
    dns_name::DNSName,
    health_check::HealthCheck,
    lb::{LBKind, TLSSettings},
    listener::Listener,
};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use trust_dns_server::proto::rr::{Name, Record, RecordSet};

fn default_ttl() -> u32 {
    30
}

// TODO trait for health checks
// TODO trait for LB generation

pub trait ToRecord {
    fn to_record(&self, config: &Config, domain: Name, serial: u32) -> Vec<RecordSet>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RecordType {
    #[serde(rename = "a", alias = "A")]
    A {
        addresses: Vec<IpAddr>,
        #[serde(default = "default_ttl")]
        ttl: u32,
        healthcheck: Vec<HealthCheck>,
    },
    #[serde(rename = "txt", alias = "TXT")]
    TXT {
        value: Vec<String>,
        #[serde(default = "default_ttl")]
        ttl: u32,
    },
    #[serde(rename = "lb", alias = "LB")]
    LB {
        backends: Vec<SocketAddr>,
        kind: LBKind,
        listeners: Vec<Listener>,
        tls: Option<TLSSettings>,
        healthcheck: Vec<HealthCheck>,
        #[serde(default = "default_ttl")]
        ttl: u32,
    },
}

fn generate_txt(domain: Name, serial: u32, value: Vec<String>, ttl: u32) -> Vec<RecordSet> {
    let mut rs = RecordSet::new(&domain, trust_dns_server::proto::rr::RecordType::TXT, ttl);

    let mut rec = Record::with(domain, trust_dns_server::proto::rr::RecordType::TXT, ttl);

    rec.set_data(Some(trust_dns_server::proto::rr::RData::TXT(
        trust_dns_server::proto::rr::rdata::TXT::new(value),
    )));

    rs.insert(rec, serial);

    vec![rs]
}

fn generate_a(domain: Name, serial: u32, addresses: Vec<IpAddr>, ttl: u32) -> Vec<RecordSet> {
    let mut v4rs = RecordSet::new(&domain, trust_dns_server::proto::rr::RecordType::A, ttl);

    for addr in addresses
        .iter()
        .filter_map(|ip| match ip {
            IpAddr::V4(ip) => Some(*ip),
            _ => None,
        })
        .collect::<Vec<Ipv4Addr>>()
    {
        let mut rec = Record::with(
            domain.clone(),
            trust_dns_server::proto::rr::RecordType::A,
            ttl,
        );
        rec.set_data(Some(trust_dns_server::proto::rr::RData::A(addr)));

        v4rs.insert(rec, serial);
    }

    let mut v6rs = RecordSet::new(&domain, trust_dns_server::proto::rr::RecordType::AAAA, ttl);

    for addr in addresses
        .iter()
        .filter_map(|ip| match ip {
            IpAddr::V6(ip) => Some(*ip),
            _ => None,
        })
        .collect::<Vec<Ipv6Addr>>()
    {
        let mut rec = Record::with(
            domain.clone(),
            trust_dns_server::proto::rr::RecordType::AAAA,
            ttl,
        );
        rec.set_data(Some(trust_dns_server::proto::rr::RData::AAAA(addr)));

        v6rs.insert(rec, serial);
    }

    vec![v4rs, v6rs]
}

impl ToRecord for RecordType {
    fn to_record(&self, config: &Config, domain: Name, serial: u32) -> Vec<RecordSet> {
        match self {
            RecordType::LB {
                backends: _,
                kind: _,
                listeners,
                tls: _,
                healthcheck: _,
                ttl,
            } => {
                let mut addresses: Option<Vec<SocketAddr>> = None;

                for listener in listeners {
                    if listener.name() == config.me {
                        addresses = listener.addr(config.clone());
                        break;
                    }
                }

                if let Some(addresses) = addresses {
                    generate_a(
                        domain,
                        serial,
                        addresses
                            .iter()
                            .map(|addr| addr.ip())
                            .collect::<Vec<IpAddr>>(),
                        *ttl,
                    )
                } else {
                    vec![]
                }
            }
            RecordType::TXT { value, ttl } => generate_txt(domain, serial, value.clone(), *ttl),
            RecordType::A {
                addresses,
                ttl,
                healthcheck: _,
            } => generate_a(domain, serial, addresses.clone(), *ttl),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SOA {
    domain: DNSName,
    admin: DNSName,
    minttl: u32,
    serial: u32,
    refresh: u32,
    retry: u32,
    expire: u32,
}

impl SOA {
    pub fn serial(&self) -> u32 {
        self.serial
    }
}

impl ToRecord for SOA {
    fn to_record(&self, _config: &Config, domain: Name, serial: u32) -> Vec<RecordSet> {
        let mut rs = RecordSet::new(
            &domain,
            trust_dns_server::proto::rr::RecordType::SOA,
            self.minttl,
        );

        let mut rec = Record::with(
            domain,
            trust_dns_server::proto::rr::RecordType::SOA,
            self.minttl,
        );

        rec.set_data(Some(trust_dns_server::proto::rr::RData::SOA(
            trust_dns_server::proto::rr::rdata::SOA::new(
                self.domain.name().clone(),
                self.admin.name().clone(),
                self.serial,
                self.refresh.try_into().unwrap(),
                self.retry.try_into().unwrap(),
                self.expire.try_into().unwrap(),
                self.minttl,
            ),
        )));

        rs.insert(rec, serial);
        vec![rs]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct NS {
    servers: Vec<DNSName>,
    #[serde(default = "default_ttl")]
    ttl: u32,
}

impl ToRecord for NS {
    fn to_record(&self, _config: &Config, domain: Name, serial: u32) -> Vec<RecordSet> {
        let mut rs = RecordSet::new(
            &domain,
            trust_dns_server::proto::rr::RecordType::NS,
            self.ttl,
        );

        for ns in &self.servers {
            let mut rec = Record::with(
                domain.clone(),
                trust_dns_server::proto::rr::RecordType::NS,
                self.ttl,
            );
            rec.set_data(Some(trust_dns_server::proto::rr::RData::NS(
                ns.name().clone(),
            )));
            rs.insert(rec, serial);
        }

        vec![rs]
    }
}

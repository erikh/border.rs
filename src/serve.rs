use crate::{config::Config, lb::LB, record_type::ToRecord};
use anyhow::anyhow;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::net::{TcpListener, UdpSocket};
use trust_dns_server::{
    authority::Catalog, client::rr::RrKey, store::in_memory::InMemoryAuthority, ServerFuture,
};

pub struct Server<'a> {
    config: &'a Config,
}

impl<'a> Server<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        self.dns().await?;

        Ok(())
    }

    pub async fn lb(&self, _lb: LB) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub async fn dns(&self) -> Result<(), anyhow::Error> {
        let sa = self.config.listen.dns;
        let tcp = TcpListener::bind(sa).await?;
        let udp = UdpSocket::bind(sa).await?;

        let mut sf = ServerFuture::new(self.construct_catalog()?);
        sf.register_socket(udp);
        sf.register_listener(tcp, Duration::new(60, 0));
        match sf.block_until_done().await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e)),
        }
    }

    fn construct_catalog(&self) -> Result<Catalog, anyhow::Error> {
        let mut catalog = Catalog::default();

        for (name, zone) in &self.config.zones {
            let mut records = BTreeMap::default();

            records.insert(
                RrKey::new(
                    name.name().into(),
                    trust_dns_server::proto::rr::RecordType::SOA,
                ),
                zone.soa
                    .to_record(self.config(), name.name().clone(), zone.soa.serial())
                    .first()
                    .expect("Expected a SOA record")
                    .clone(),
            );

            let ns_records =
                zone.ns
                    .to_record(self.config(), name.name().clone(), zone.soa.serial());
            for record in ns_records {
                records.insert(
                    RrKey::new(
                        name.name().into(),
                        trust_dns_server::proto::rr::RecordType::NS,
                    ),
                    record,
                );
            }

            for zonerec in &zone.records {
                let rec = zonerec.record.to_record(
                    self.config(),
                    zonerec.name.name().clone(),
                    zone.soa.serial(),
                );

                for rectype in rec {
                    records.insert(
                        RrKey::new(zonerec.name.name().into(), rectype.record_type()),
                        rectype,
                    );
                }
            }

            let authority = InMemoryAuthority::new(
                name.name().clone(),
                records,
                trust_dns_server::authority::ZoneType::Primary,
                false,
            )
            .unwrap();

            catalog.upsert(name.name().into(), Box::new(Arc::new(authority)));
        }

        Ok(catalog)
    }

    pub fn config(&self) -> &Config {
        self.config
    }
}

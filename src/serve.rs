use std::{collections::BTreeMap, sync::Arc};

use trust_dns_server::{
    authority::Catalog, client::rr::RrKey, store::in_memory::InMemoryAuthority,
};

use crate::{config::Config, record_type::ToRecord};

pub struct Server<'a> {
    config: &'a Config,
}

impl Server<'_> {
    pub fn construct_catalog(&self) -> Result<Catalog, anyhow::Error> {
        let mut catalog = Catalog::default();

        for (name, zone) in &self.config.zones {
            let mut records = BTreeMap::default();

            records.insert(
                RrKey::new(
                    name.name().into(),
                    trust_dns_server::proto::rr::RecordType::SOA,
                ),
                zone.soa
                    .to_record(name.name().clone(), zone.soa.serial())
                    .first()
                    .expect("Expected a SOA record")
                    .clone(),
            );

            let ns_records = zone.ns.to_record(name.name().clone(), zone.soa.serial());
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
                let rec = zonerec
                    .record
                    .to_record(zonerec.name.name().clone(), zone.soa.serial());

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

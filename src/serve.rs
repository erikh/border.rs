use crate::{
    config::{Config, Record},
    lb::LB,
    record_type::{RecordType, ToRecord},
};
use anyhow::anyhow;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    net::{TcpListener, UdpSocket},
    sync::Mutex,
};
use trust_dns_server::{
    authority::Catalog, client::rr::RrKey, store::in_memory::InMemoryAuthority, ServerFuture,
};

#[derive(Clone)]
pub struct Server {
    config: Arc<Mutex<Config>>,
    context: Arc<AtomicBool>,
}

impl Server {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        let context = Arc::new(AtomicBool::default());
        Self { config, context }
    }

    pub fn shutdown(&self) {
        self.context.store(true, Ordering::Relaxed)
    }

    pub async fn start(&self) -> Result<(), anyhow::Error> {
        let zones = &self.config.lock().await.zones.clone();
        for (_, zone) in zones {
            let records = zone
                .records
                .iter()
                .filter_map(|rec| {
                    match rec.record {
                        RecordType::LB { .. } => return Some(rec.clone()),
                        _ => {}
                    }
                    None
                })
                .collect::<Vec<Record>>();

            for record in &records {
                let lb = LB::new(self.config.clone(), record.record.clone())?;
                let context = self.context.clone();
                tokio::spawn(async move { lb.serve(context).await.unwrap() });
            }
        }

        let obj = self.clone();
        let handle = tokio::spawn(async move { obj.dns().await.unwrap() });

        loop {
            if self.context.load(Ordering::Relaxed) {
                handle.abort();
                break;
            }

            tokio::time::sleep(std::time::Duration::new(0, 1000000)).await;
        }

        Ok(())
    }

    pub async fn dns(&self) -> Result<(), anyhow::Error> {
        let sa = self.config.lock().await.listen.dns.clone();
        let tcp = TcpListener::bind(sa).await?;
        let udp = UdpSocket::bind(sa).await?;

        let mut sf = ServerFuture::new(self.construct_catalog().await?);
        sf.register_socket(udp);
        sf.register_listener(tcp, Duration::new(60, 0));
        match sf.block_until_done().await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow!(e)),
        }
    }

    async fn construct_catalog(&self) -> Result<Catalog, anyhow::Error> {
        let mut catalog = Catalog::default();
        let zones = &self.config.lock().await.zones.clone();

        for (name, zone) in zones {
            let mut records = BTreeMap::default();

            records.insert(
                RrKey::new(
                    name.name().into(),
                    trust_dns_server::proto::rr::RecordType::SOA,
                ),
                zone.soa
                    .to_record(self.config.clone(), name.name().clone(), zone.soa.serial())
                    .await
                    .first()
                    .expect("Expected a SOA record")
                    .clone(),
            );

            let ns_records = zone
                .ns
                .to_record(self.config.clone(), name.name().clone(), zone.soa.serial())
                .await;
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
                    .to_record(
                        self.config.clone(),
                        zonerec.name.name().clone(),
                        zone.soa.serial(),
                    )
                    .await;

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
}

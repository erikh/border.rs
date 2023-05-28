use std::{
    collections::BTreeMap,
    net::SocketAddr,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

use anyhow::anyhow;
use hyper::{
    client::HttpConnector,
    http::{
        uri::{Authority, Scheme},
        HeaderValue,
    },
    service::{make_service_fn, service_fn},
    Body, Client, Request, Response, Server, Uri,
};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use crate::{config::SafeConfig, record_type::RecordType};

const HEADER_X_FORWARDED_FOR: &str = "X-Forwarded-For";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LBKind {
    #[serde(rename = "tcp", alias = "TCP")]
    TCP,
    #[serde(rename = "http", alias = "HTTP")]
    HTTP,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TLSSettings {
    certificate: String,
    key: String,
}

pub struct LB {
    config: SafeConfig,
    record: RecordType,
}

struct StreamContainer(TcpStream);

#[derive(Default)]
struct BackendCount(BTreeMap<SocketAddr, AtomicU64>);

impl BackendCount {
    pub fn finished(&mut self, backend: SocketAddr) {
        self.0
            .get(&backend)
            .unwrap()
            .fetch_sub(1, Ordering::Acquire);
    }

    pub async fn get_backend(&mut self, backends: Vec<SocketAddr>) -> SocketAddr {
        let mut lowest_backend: Option<SocketAddr> = None;
        let lowest_backend_count = AtomicU64::default();

        for backend in &backends {
            match self.0.get(backend) {
                Some(count) => {
                    if count.load(Ordering::Relaxed) <= lowest_backend_count.load(Ordering::Relaxed)
                    {
                        lowest_backend = Some(*backend);
                        lowest_backend_count.store(count.load(Ordering::Acquire), Ordering::SeqCst)
                    }
                }
                None => {
                    lowest_backend = Some(*backend);
                    lowest_backend_count.store(0, Ordering::SeqCst);
                }
            }
        }

        if let None = lowest_backend {
            lowest_backend = Some(
                *backends
                    .iter()
                    .nth(0)
                    .expect("Could not find any backends to service"),
            );

            lowest_backend_count.store(
                self.0
                    .get(&lowest_backend.unwrap())
                    .unwrap()
                    .load(Ordering::Acquire),
                Ordering::SeqCst,
            );
        }

        let backend = lowest_backend.unwrap();

        if self.0.contains_key(&backend) {
            self.0
                .get(&backend)
                .unwrap()
                .fetch_add(1, Ordering::Acquire);
        } else {
            self.0.insert(backend, lowest_backend_count);
        }

        backend
    }
}

impl LB {
    pub fn new(config: SafeConfig, record: RecordType) -> Result<Self, anyhow::Error> {
        match record {
            RecordType::LB { .. } => {}
            _ => return Err(anyhow!("Record type was not LB")),
        }

        Ok(Self { config, record })
    }

    async fn listen_addrs(&self) -> Result<Option<Vec<SocketAddr>>, anyhow::Error> {
        match &self.record {
            RecordType::LB { listeners, .. } => {
                let mut addresses: Option<Vec<SocketAddr>> = None;

                for listener in listeners {
                    if listener.name() == self.config.lock().await.me {
                        addresses = listener.addr(self.config.clone()).await;
                        break;
                    }
                }

                Ok(addresses)
            }
            _ => Err(anyhow!("Record type was not LB")),
        }
    }

    fn backends(&self) -> Result<Vec<SocketAddr>, anyhow::Error> {
        match &self.record {
            RecordType::LB { backends, .. } => Ok(backends.clone()),
            _ => Err(anyhow!("Record type was not LB")),
        }
    }

    fn kind(&self) -> Result<LBKind, anyhow::Error> {
        match &self.record {
            RecordType::LB { kind, .. } => Ok(kind.clone()),
            _ => Err(anyhow!("Record type was not LB")),
        }
    }

    async fn serve_http(&self, context: Arc<AtomicBool>) -> Result<(), anyhow::Error> {
        let addresses = self.listen_addrs().await?.expect("No addresses to bind to");

        for address in addresses {
            tokio::spawn(Self::serve_http_listener(
                context.clone(),
                self.backends()?,
                address,
            ));
        }

        Ok(())
    }

    async fn http_handler(
        backends: Vec<SocketAddr>,
        backend_count: Arc<Mutex<BackendCount>>,
        address: SocketAddr,
        client: Arc<Client<HttpConnector>>,
        req: Request<Body>,
    ) -> Result<Response<Body>, anyhow::Error> {
        let mut headers = req.headers().clone();

        if let Some(xff) = headers.get(HEADER_X_FORWARDED_FOR) {
            headers.insert(
                HEADER_X_FORWARDED_FOR,
                HeaderValue::from_str(&format!(
                    "{},{}",
                    address.ip().to_string(),
                    xff.clone().to_str().unwrap(),
                ))
                .unwrap(),
            );
        } else {
            headers.insert(
                HEADER_X_FORWARDED_FOR,
                HeaderValue::from_str(&address.ip().to_string()).unwrap(),
            );
        }

        let backend = backend_count.lock().await.get_backend(backends).await;

        let mut uri_parts = req.uri().clone().into_parts();
        uri_parts.scheme = Some(Scheme::HTTP);
        uri_parts.authority = Some(Authority::from_str(&backend.to_string()).unwrap());
        let uri = Uri::from_parts(uri_parts).unwrap();

        let (mut parts, body) = req.into_parts();
        parts.uri = uri;
        parts.headers = headers.clone();

        let newreq = Request::from_parts(parts, body);

        let res = client.request(newreq).await;

        tokio::spawn(async move {
            backend_count.lock().await.finished(backend);
        });

        match res {
            Ok(resp) => Ok(resp),
            Err(_) => Ok(Response::builder().status(403).body(Body::empty()).unwrap()),
        }
    }

    async fn serve_http_listener(
        context: Arc<AtomicBool>,
        backends: Vec<SocketAddr>,
        address: SocketAddr,
    ) -> Result<(), anyhow::Error> {
        let backends = Arc::new(backends);
        let backend_count = Arc::new(Mutex::new(BackendCount::default()));

        let mut connector = HttpConnector::new();
        connector.set_reuse_address(true);
        connector.set_keepalive(Some(std::time::Duration::new(1, 0)));

        let client = Arc::new(
            Client::builder()
                .pool_idle_timeout(None)
                .http1_title_case_headers(true)
                .build(connector),
        );

        let service = make_service_fn(move |_conn| {
            let backend_count = backend_count.clone();
            let backends = backends.clone();
            let client = client.clone();
            let service = service_fn(move |req| {
                Self::http_handler(
                    backends.clone().to_vec(),
                    backend_count.clone(),
                    address,
                    client.clone(),
                    req,
                )
            });

            async move { Ok::<_, anyhow::Error>(service) }
        });

        let handle = tokio::spawn(Server::bind(&address).serve(service));

        loop {
            if context.load(Ordering::Relaxed) {
                handle.abort();
                break;
            }

            tokio::time::sleep(std::time::Duration::new(0, 1000000)).await;
        }

        Ok(())
    }

    async fn serve_tcp(&self, context: Arc<AtomicBool>) -> Result<(), anyhow::Error> {
        let addresses = self.listen_addrs().await?.expect("No addresses to bind to");

        for address in addresses {
            tokio::spawn(Self::serve_tcp_listener(
                context.clone(),
                self.backends()?,
                address,
            ));
        }

        Ok(())
    }

    async fn serve_tcp_listener(
        context: Arc<AtomicBool>,
        mut backends: Vec<SocketAddr>,
        address: SocketAddr,
    ) -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind(address).await?;
        let backend_count = Arc::new(Mutex::new(BackendCount::default()));

        loop {
            if context.load(Ordering::Relaxed) {
                return Ok(());
            }

            let socket = listener.accept().await?;

            'retry: loop {
                let backend = backend_count
                    .lock()
                    .await
                    .get_backend(backends.clone())
                    .await;

                match TcpStream::connect(backend).await {
                    Ok(stream) => {
                        let socket = Arc::new(Mutex::new(Box::new(socket)));
                        let stream = Arc::new(Mutex::new(Box::new(StreamContainer(stream))));
                        let backend_count = backend_count.clone();

                        tokio::spawn(async move {
                            match tokio::io::copy_bidirectional(
                                &mut socket.lock().await.0,
                                &mut stream.lock().await.0,
                            )
                            .await
                            {
                                Ok(_) => {}
                                Err(_) => {}
                            }

                            backend_count.lock().await.finished(backend);
                        });
                    }
                    // FIXME logging
                    Err(e) => {
                        eprintln!("{}", e);
                        backends.retain(|be| *be != backend);
                        continue 'retry;
                    }
                };

                break;
            }
        }
    }

    pub async fn serve(&self, context: Arc<AtomicBool>) -> Result<(), anyhow::Error> {
        match self.kind() {
            Ok(LBKind::TCP) => self.serve_tcp(context).await,
            Ok(LBKind::HTTP) => self.serve_http(context).await,
            Err(e) => Err(e),
        }
    }
}

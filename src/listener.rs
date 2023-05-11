use serde::{de::Visitor, Deserialize, Serialize};
use std::net::SocketAddr;

use crate::config::Config;

#[derive(Debug, Default)]
pub struct Listener(String, u16);

impl Listener {
    pub fn name(&self) -> String {
        self.0.clone()
    }

    pub fn port(&self) -> u16 {
        self.1
    }

    #[allow(dead_code)]
    pub fn addr(&self, c: Config) -> Option<Vec<SocketAddr>> {
        for peer in c.peers {
            if peer.name() == self.name() {
                return Some(
                    peer.ips
                        .iter()
                        .map(|ip| SocketAddr::new(*ip, self.port()))
                        .collect::<Vec<SocketAddr>>(),
                );
            }
        }
        None
    }
}

impl Serialize for Listener {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}:{}", self.name(), self.port()))
    }
}

struct ListenerVisitor;

impl Visitor<'_> for ListenerVisitor {
    type Value = Listener;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expecting a peer:port")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let parts = v.splitn(2, ':').collect::<Vec<&str>>();
        let port: u16 = match parts[1].parse() {
            Ok(res) => res,
            Err(e) => return Err(serde::de::Error::custom(e)),
        };

        Ok(Listener(parts[0].to_string(), port))
    }
}

impl<'de> Deserialize<'de> for Listener {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ListenerVisitor)
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum LBKind {
    #[serde(rename = "tcp", alias = "TCP")]
    TCP,
    #[serde(rename = "http", alias = "HTTP")]
    HTTP,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TLSSettings {
    certificate: String,
    key: String,
}


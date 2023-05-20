use serde::{Deserialize, Serialize};

use crate::record_type::RecordType;

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

#[allow(dead_code)]
pub struct LB {
    record: RecordType,
}

use josekit::jwk::Jwk;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub auth_key: Jwk,
    pub base_url: Url,
}

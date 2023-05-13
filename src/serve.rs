use crate::config::Config;

pub struct Server<'a> {
    config: &'a Config,
}

impl Server<'_> {
    pub fn config(&self) -> &Config {
        self.config
    }
}

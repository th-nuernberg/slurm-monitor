use std::{env, net::IpAddr};

use collector_data::{misc::parsing::Duration, DEFAULT_INTERVAL};
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

// TODO put into collector_data (and backend too)
const DEFAULT_SERVER_IP: &str = "127.0.0.1";
const DEFAULT_SERVER_PORT: u16 = 19912;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server_addr: IpAddr,
    pub server_port: u16,
    pub tx_interval: Option<Duration>,
}

// TODO (important) remove this messy config crate, just use clap
// TODO (important) accept hostname (kiz0 e.g.)
impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "default".into());

        let builder = Config::builder()
            .set_default("server_ip", DEFAULT_SERVER_IP.to_string())?
            .set_default("server_port", DEFAULT_SERVER_PORT.to_string())?
            //.add_source(File::with_name("config"))
            //.add_source(File::with_name(&format!("config/{}", run_mode)))
            .add_source(Environment::default())
            .build()?;

        let mut settings = builder.try_deserialize::<Self>()?;
        settings.tx_interval = settings.tx_interval.or(Some(Duration(DEFAULT_INTERVAL)));
        Ok(settings)
    }
}

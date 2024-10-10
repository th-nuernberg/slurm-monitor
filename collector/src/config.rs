use std::{env, net::IpAddr};

use collector_data::{misc::parsing::Duration, DEFAULT_INTERVAL};
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

const DEFAULT_SERVER_IP: &str = "127.0.0.1";
const DEFAULT_SERVER_SOCKET: u16 = 6430;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server_ip: IpAddr,
    pub server_socket: u16,
    pub tx_interval: Option<Duration>,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "default".into());

        let builder = Config::builder()
            .set_default("server_ip", DEFAULT_SERVER_IP.to_string())?
            .set_default("server_socket", DEFAULT_SERVER_SOCKET.to_string())?
            .add_source(File::with_name("config"))
            .add_source(File::with_name(&format!("config/{}", run_mode)))
            .add_source(Environment::with_prefix("app"))
            .build()?;

        let mut settings = builder.try_deserialize::<Self>()?;
        settings.tx_interval = settings.tx_interval.or(Some(Duration(DEFAULT_INTERVAL)));
        Ok(settings)
    }
}

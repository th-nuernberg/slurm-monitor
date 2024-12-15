use chrono::{DateTime, Duration, Local};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::sync::Mutex;

use collector_data::{DEFAULT_INTERVAL, DEFAULT_TIMEOUT};

/// Stores things like when we last saw the client. Used to determine timeouts.
#[derive(Debug, Clone)]
pub struct ClientMetadata {
    pub last_recv: DateTime<Local>,
    pub interval: Duration,
    pub timeout: Duration,
}

/// https://stackoverflow.com/questions/50282619/is-it-possible-to-share-a-hashmap-between-threads-without-locking-the-entire-has
pub type ClientMap = Arc<Mutex<HashMap<IpAddr, ClientMetadata>>>;

impl ClientMetadata {
    pub fn new() -> Self {
        ClientMetadata {
            last_recv: Local::now(),
            interval: DEFAULT_INTERVAL,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn update_last_recv(&mut self) {
        self.last_recv = Local::now();
    }

    pub fn has_timed_out_since(&self, when: Option<DateTime<Local>>) -> bool {
        when.unwrap_or_else(Local::now) - self.last_recv > self.interval + self.timeout
    }

    pub fn has_timed_out(&self) -> bool {
        self.has_timed_out_since(Some(Local::now()))
    }
}

impl Default for ClientMetadata {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod test {

    use chrono::{Duration, Local};
    use color_eyre::Result;

    use super::*;

    #[test]
    fn ClientMetadata__has_timed_out__yes() -> Result<()> {
        let client = ClientMetadata {
            last_recv: Local::now() - Duration::seconds(65),
            interval: Duration::seconds(30),
            timeout: Duration::seconds(30),
        };
        assert!(client.has_timed_out());
        Ok(())
    }

    #[test]
    fn ClientMetadata__has_timed_out__no() -> Result<()> {
        let mut client = ClientMetadata {
            last_recv: Local::now() - Duration::seconds(20),
            interval: Duration::seconds(30),
            timeout: Duration::seconds(30),
        };
        assert!(!client.has_timed_out());
        client.last_recv = Local::now() - Duration::seconds(40);
        assert!(!client.has_timed_out());
        Ok(())
    }
}

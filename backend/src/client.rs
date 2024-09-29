use std::{collections::HashMap, net::SocketAddr};
use std::time::{Duration, Instant};
use std::sync::{Mutex, Arc};

use collector_data::{DEFAULT_INTERVAL, DEFAULT_TIMEOUT};

use super::config::ClientConfig;

/// Stores things like when we last saw the client. Used to determine timeouts.
#[derive(Debug, Clone)]
pub struct ClientMetadata {
    pub last_recv: Instant,
    pub interval: Duration,
    pub timeout: Duration,
}

/// https://stackoverflow.com/questions/50282619/is-it-possible-to-share-a-hashmap-between-threads-without-locking-the-entire-has
pub type ClientMap = Arc<Mutex<HashMap<SocketAddr, ClientMetadata>>>;

impl ClientMetadata {
    pub fn new() -> Self {
        ClientMetadata {
            last_recv: Instant::now(),
            interval: DEFAULT_INTERVAL,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn update_last_recv(&mut self) {
        self.last_recv = Instant::now();
    }

    pub fn has_timed_out_since(&self, when: Option<Instant>) -> bool {
        when.unwrap_or_else(|| Instant::now()) - self.last_recv > self.interval + self.timeout
    }

    pub fn has_timed_out(&self) -> bool {
        self.has_timed_out_since(Some(Instant::now()))
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
    use anyhow::Result;
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn ClientMetadata__has_timed_out__yes() -> Result<()> {
        let client = ClientMetadata {
            last_recv: Instant::now() - Duration::from_secs(65),
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(30)
        };
        assert!(client.has_timed_out());
        Ok(())
    }

    #[test]
    fn ClientMetadata__has_timed_out__no() -> Result<()> {
        let mut client = ClientMetadata {
            last_recv: Instant::now() - Duration::from_secs(20),
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(30)
        };
        assert!(!client.has_timed_out());
        client.last_recv = Instant::now() - Duration::from_secs(40);
        assert!(!client.has_timed_out());
        Ok(())
    }
}
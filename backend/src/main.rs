use anyhow::{Ok, Result};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

const POLL_INTERVAL: Duration = Duration::from_secs(10);

struct AbortHandler {
    atom: Arc<AtomicBool>,
}

impl AbortHandler {
    pub fn new() -> Result<Self> {
        let result = Self {
            atom: Arc::new(AtomicBool::new(false)),
        };

        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&result.atom))?;
        signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&result.atom))?;

        Ok(result)
    }

    pub fn abort(&self) -> bool {
        self.atom.load(Ordering::Relaxed)
    }
}

fn main() -> Result<()> {
    setup();
    let abort_handler = AbortHandler::new()?;

    while !abort_handler.abort() {
        load_data();
        sleep(POLL_INTERVAL);
    }

    Ok(())
}

fn load_data() -> _ {
    todo!()
}

fn setup() {
    todo!()
}

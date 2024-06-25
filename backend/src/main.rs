mod cli;
pub mod collect;

use anyhow::{ensure, Context, Ok, Result};
use clap::Parser as _;
use cli::Args;
use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

const POLL_INTERVAL: Duration = Duration::from_secs(60);

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
    let args = Args::parse();
    let abort_handler = AbortHandler::new()?;

    setup(&args)?;
    while !abort_handler.abort() {
        collect(&args.data_dir)?;
        sleep(POLL_INTERVAL);
    }

    Ok(())
}

fn collect(data_dir: impl AsRef<Path>) -> Result<()> {
    let dataset: HashMap<_, _> = [(
        "sacct",
        collect::collect_sacct_json().unwrap_or_else(|e| {
            eprintln!("Couldn't collect `sacct`: {e}");
            "".to_owned()
        }),
    )]
    .into_iter()
    .collect();

    for (what, data) in dataset.iter() {
        let filename = data_dir.as_ref().join(gen_filename(what));
        let mut file = File::create_new(filename)?;
        file.write_all(data.as_bytes())?;
    }

    Ok(())
}

fn gen_filename(what: &str) -> String {
    let datetime = chrono::Local::now().format("%Y_%m_%d__%H_%M_%S_%3f");
    format!("{datetime}__{what}")
}

fn setup(args: &Args) -> Result<()> {
    // test data dir
    if !args.data_dir.exists() {
        std::fs::create_dir_all(&args.data_dir).context("creating data dir")?;
    }
    ensure!(args.data_dir.exists() && args.data_dir.is_dir());

    Ok(())
}

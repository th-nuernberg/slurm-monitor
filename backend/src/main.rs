mod cli;
pub mod collect;
pub mod client;

use anyhow::{bail, ensure, Context, Result};
use async_std::{io::ReadExt, net::{TcpListener, TcpStream}};
use clap::Parser as _;
use cli::Args;
use client::{ClientMetadata, ClientMap};
use futures::StreamExt;
use tracing::{error, info, instrument, span, Instrument, Level};
use std::{
    collections::HashMap, fs::File, io::Write, net::{IpAddr, SocketAddr}, path::Path, sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    }, thread::sleep, time::Duration
};

const TIMEOUT_POLL_INTERVAL: Duration = Duration::from_secs(5);

#[instrument]
#[async_std::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let abort_handler = AbortHandler::new()?;

    register_logging(args.log_level);
    open_database(args.database)?;

    let mut connections = ClientMap::default();

    let socket = TcpListener::bind((args.ip, args.port)).await?;

    let listen_future = socket.incoming().for_each_concurrent(None, |stream| {
        let connections = connections.clone();
        async move {
            let stream = match stream {
                Ok(stream) => stream,
                Err(e) => {error!(?e, "Unwrapping TcpStream failed"); return},
            };
            let client_addr = match stream.peer_addr() {
                Ok(addr) => addr,
                Err(e) => {error!(?e, "Extracting client SocketAddr failed"); return},
            };

            handle_connection(stream, client_addr).await;
            update_last_recv(connections, client_addr).map_err(|e| error!(?e, "Could not update client metadata"));
    }}.instrument(span!(Level::TRACE, "socket stream closure")));

    /*while !abort_handler.abort() {
        let (stream, client_addr) = socket.accept().await?;
        let client = connections.entry(client_addr).or_insert(Client::new());

        let recv_data = stream.read_to_end(buf);
        client.update_last_received();

        todo!() // read from clients, update connection map, store the data (in .json.gz or sth should be smartest)
    }*/

    Ok(())
}

fn register_logging(level: Option<Level>) -> Result<()> {
    // a builder for `FmtSubscriber`.
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(level.unwrap_or(Level::INFO))
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("setting default subscriber failed")
}

#[instrument]
fn open_database(path: &Path) -> Result<native_db::Database> {
    
}

#[instrument]
async fn handle_connection(mut stream: TcpStream, client_addr: SocketAddr) -> Result<()> {
    let mut buf = Vec::<u8>::new();
    stream.read_to_end(&mut buf).await.map(|len| {info!(len, "rx({client_addr})")}).context("reading from TcpStream")?;
    
    

    Ok(())
}

#[instrument]
fn update_last_recv(connections: ClientMap, client_addr: SocketAddr) -> Result<()> {
    let mut connections =match  connections.lock()  {
        Ok(guard) => guard,
        Err(e) =>{ 
        bail!("Error locking mutex: {e:#?}");},
    };
    connections.entry(client_addr).or_default().update_last_recv();

    Ok(())
}


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


/*fn collect(data_dir: impl AsRef<Path>) -> Result<()> {
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
}o/

fn gen_filename(what: &str) -> String {
    let datetime = chrono::Local::now().format("%Y_%m_%d__%H_%M_%S_%3f");
    format!("{datetime}__{what}.json")
}

fn setup(args: &Args) -> Result<()> {
    // test data dir
    if !args.data_dir.exists() {
        std::fs::create_dir_all(&args.data_dir).context("creating data dir")?;
    }
    ensure!(args.data_dir.exists() && args.data_dir.is_dir());

    Ok(())
}
*/
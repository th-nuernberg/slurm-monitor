mod cli;
pub mod client;

use anyhow::{bail, ensure, Context, Result};
use tokio::{fs::{read_to_string, File}, io::{AsyncReadExt as _, AsyncWriteExt as _, BufWriter}, net::{TcpListener, TcpStream}, sync::{mpsc::{unbounded_channel, UnboundedSender}, Mutex}, task::JoinHandle};
use clap::Parser as _;
use cli::Args;
use client::{ClientMetadata, ClientMap};
use collector_data::monitoring_info::DataObject;
use futures::{StreamExt, TryFutureExt};
use tracing::{error, info, instrument, span, warn, Instrument, Level};
use std::{
    collections::HashMap, future::Future, io::Write, mem::size_of, net::{IpAddr, SocketAddr}, path::Path, sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock,
    }, time::Duration
};

const CHECK_STALE_INTERVAL: Duration = Duration::from_secs(5);

#[instrument]
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let data_dir = &args.data_dir;



let client_map: ClientMap = Arc::new(Mutex::new(HashMap::new()));
    let abort_handler = AbortHandler::new()?;

    register_logging(args.log_level)?;

    let check_stale_worker = start_check_stale_worker(client_map.clone(), CHECK_STALE_INTERVAL, abort_handler.clone());
    let (save_worker_handle, save_tx) = start_save_worker(data_dir, abort_handler.clone())?;

    let socket = TcpListener::bind((args.ip, args.port)).await?;

    while !abort_handler.abort() {
        let connection = socket.accept().await;
        tokio::spawn({let save_tx = save_tx.clone();
            let client_map = client_map.clone();
            async move {
                handle_connection(connection, save_tx, client_map).await.map_err(|e| error!("error in handle_connection: {e:#}"))
        }}.instrument(span!(Level::TRACE, "socket stream closure")));
    };

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
async fn start_check_stale_worker(connections: ClientMap, interval: Duration, abort_handler: AbortHandler) -> JoinHandle<()> {
    use tokio::time::interval as new_interval;
    let mut interval = new_interval(interval);
    tokio::spawn(async move {
        while abort_handler.abort() {
            interval.tick().await;
            
    for (client, metadata) in connections.lock().await.iter() {
        if metadata.has_timed_out() {
            warn!("{client}: Client timed out (last seen {})", metadata.last_recv);
        }
    }
    }})

}

// FIXME maybe sending raw bytes over network is unsafe (endianness and stuff). (But since this is Unicode, we should be ok?)
#[instrument]
async fn handle_connection(new_connection: std::io::Result<(TcpStream, SocketAddr)>, save_tx: UnboundedSender<DataObject>, mut client_map: ClientMap) -> Result<()> {
    let (mut stream, client_addr) = new_connection?;

    let mut buf = Vec::<u8>::with_capacity(size_of::<DataObject>());  // reserve space for one object
    stream.read_to_end(&mut buf).await.map(|len: usize| {info!(len, "rx({client_addr})")}).with_context(|| format!("reading TcpStream from {client_addr}"))?;

    let packet: DataObject = serde_json::from_slice(&buf)?;
    save_tx.send(packet).with_context(||format!("trying to save packet from {client_addr}"))?;

    let _ = update_last_recv(client_addr, &mut client_map).map_err(|e| error!(?e, "Could not update client metadata")).await;  // client metadata is noncritical, so we don't fail here

    Ok(())
}

/// currently, this saves a data object by appending it to a list inside a JSON,
/// grouped by the date. (E.g. file 2014-09-09 holds every data object collected that day)
/// 
/// This needs to run in its own task, to synchronize writing to FS.
#[instrument]
fn start_save_worker(path: &Path, abort_handler: AbortHandler) -> Result<(JoinHandle<Result<()>>, UnboundedSender<DataObject>)>{
    let (tx, mut rx) = unbounded_channel();
    let path = path.to_path_buf();

    let handle = tokio::spawn(async move {
    // TODO TEST (especially appending and naming behaviour)
    while !abort_handler.abort() {
        // TODO PERFORMANCE when we receive more data, check out channel::recv_many().
        let Some(packet) : Option<DataObject>= rx.recv().await else {
            return Ok(())
        };
        let filename = path.join(packet.time.format("%Y-%m-%d").to_string());
    
        let mut all_objects: Vec<DataObject> = serde_json::from_str(&read_to_string(&path).await?)?;
        all_objects.push(packet);
    
        let mut writer = BufWriter::new(File::create(filename).await?);
        writer.write_all(&serde_json::to_vec_pretty(&all_objects)?).await?
    }
Ok(())});

    Ok((handle, tx))
}

#[instrument]
async fn update_last_recv(client_addr: SocketAddr, client_map: &mut ClientMap) -> Result<()> {
    let mut client_map = client_map.lock().await;
    client_map.entry(client_addr).or_default().update_last_recv();

    Ok(())
}

#[derive(Debug, Clone)]
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
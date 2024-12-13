mod cli;
pub mod client;

use async_compression::tokio::write::BrotliEncoder;
use clap::Parser as _;
use cli::Args;
use client::ClientMap;
use collector_data::monitoring_info::Measurement;
use color_eyre::eyre::{bail, eyre, Context, Report, Result};
use futures::{join, pin_mut, StreamExt as _, TryFutureExt as _};
use itertools::Itertools;
use serde::Deserialize;
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    fs::File,
    io::{AsyncReadExt as _, AsyncWriteExt as _, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{error::TryRecvError, unbounded_channel, UnboundedSender},
        Mutex,
    },
    task::JoinHandle,
    time::sleep,
};
use tracing::{debug, debug_span, error, field, info, instrument, span, trace, warn, Instrument, Level, Span};

const CHECK_STALE_INTERVAL: Duration = Duration::from_secs(5);
const SAVE_FILE_EXT: &str = "json.br";
const MAX_SAVE_PACKETS_AT_ONCE: usize = 1024;
const BROTLI_LEVEL: i32 = 5; // only like 10% worse than 9 while taking 35% of the time

/// `whorker_threads`: only workers for async tasks (tokio::spawn, main). spawn_blocking spawns extra threads
#[tokio::main(worker_threads = 4)]
async fn main() -> Result<()> {
    let args = Args::parse();
    let data_dir = &args.data_dir;

    let client_map: ClientMap = Arc::new(Mutex::new(HashMap::new()));
    let abort_handler = AbortHandler::new()?;

    register_logging(args.log_level)?;

    let _check_stale_worker_handle = start_check_stale_worker(client_map.clone(), CHECK_STALE_INTERVAL, abort_handler.clone());
    let (save_worker_handle, save_tx) = start_save_worker(data_dir, abort_handler.clone())?;

    let socket = TcpListener::bind((args.ip, args.port)).await?;
    let socket_stream = futures::stream::poll_fn(move |ctx| socket.poll_accept(ctx).map(Option::from)); // TODO (maybe) find out if the closures context arg is relevant to us (and what it means in general)
    let (mut socket_stream, socket_stream_abort_handler) = futures::stream::abortable(socket_stream);

    tokio::spawn({
        let abort_handler = abort_handler.clone();
        async move {
            // TODO prob. duplicate to `abortable()`
            while !abort_handler.abort() {
                let Some(connection) = socket_stream.next().await else {
                    eprintln!("Collector socket closed; exiting…");
                    break;
                };
                tokio::spawn(
                    {
                        let save_tx = save_tx.clone();
                        let client_map = client_map.clone();
                        async move {
                            handle_connection(connection, save_tx, client_map)
                                .await
                                .map_err(|e| error!("error in handle_connection: {e:#}"))
                        }
                    }
                    .instrument(span!(Level::TRACE, "handle_connection closure")),
                );
            }
            socket_stream_abort_handler.abort();
        }
        .instrument(span!(Level::TRACE, "main connection loop"))
    });

    // wait for abort
    while !abort_handler.abort() {
        trace!("waiting for abort_handler");

        // if save_worker crashed (or finished, but usually crashed) for some reason, end prematurely
        if save_worker_handle.is_finished() {
            break;
        }

        sleep(Duration::from_millis(100)).await;
    }

    // TODO into scope guard or, better, object with `drop()`
    //
    // we only join on save_worker, because checking for stale clients is irrelevant at this point, and we explicitly
    // want the `accept()` loop to quit. Downside is hypothetical abortion of open connections, but that would require
    // using the `JoinHandle`s from the nested `spawn()`
    let _ = join!(save_worker_handle).0.map_err(Report::new).and_then(|x| x).map_err(|e| {
        error!("save worker crashed: {e:#}");
    }); // wait for save worker, unwrap if ok (flatten())

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

    tracing::subscriber::set_global_default(subscriber).context("setting default subscriber failed")
}

#[tracing::instrument]
fn start_check_stale_worker(connections: ClientMap, interval: Duration, abort_handler: AbortHandler) -> JoinHandle<()> {
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
        }
    })
}

// FIXME maybe sending raw bytes over network is unsafe (endianness and stuff). (But since this is Unicode, we should be ok?)
#[instrument(skip(save_tx, client_map))]
async fn handle_connection(
    new_connection: std::io::Result<(TcpStream, SocketAddr)>,
    save_tx: UnboundedSender<Measurement>,
    mut client_map: ClientMap,
) -> Result<()> {
    let (mut stream, client_addr) = new_connection?;

    let mut buf = Vec::<u8>::with_capacity(size_of::<Measurement>()); // reserve space for one object
    stream
        .read_to_end(&mut buf)
        .await
        .map(|len: usize| info!(len, "rx({client_addr})"))
        .with_context(|| format!("reading TcpStream from {client_addr}"))?;

    let packet: Measurement = serde_json::from_slice(&buf)?;
    save_tx
        .send(packet)
        .with_context(|| format!("trying to save packet from {client_addr}"))?;

    let _ = update_last_recv(client_addr, &mut client_map)
        .map_err(|e| error!(?e, "Could not update client metadata"))
        .await; // client metadata is noncritical, so we don't fail here

    Ok(())
}

/// currently, this saves a data object by appending it to a list inside a JSON,
/// grouped by the date. (E.g. file 2014-09-09 holds every data object collected that day)
///
/// This needs to run in its own task, to synchronize writing to FS.
// FIXME (important) check for errors && restart while main loop is running. (Currently, join! only checks at the end)
fn start_save_worker(path: &Path, abort_handler: AbortHandler) -> Result<(JoinHandle<Result<()>>, UnboundedSender<Measurement>)> {
    let (tx, mut rx) = unbounded_channel();
    let path = path.to_path_buf();

    let handle = tokio::spawn(async move {
        // TODO TEST (especially appending and naming behaviour)
        // TODO profile & write timings to logs
        while !abort_handler.abort() {
            // we need the async block, so we can correctly instrument with a span later. Using Span::enter
            // doesn't work with async.
            async {
                // TODO PERFORMANCE when we receive more data, check out channel::recv_many().
                // TODO use Abortable and make this simpler snippet work again
                /*let Some(packet): Option<DataObject> = rx.recv().await else {
                    return Ok(());
                };*/
                let mut recv_buf: Vec<Measurement> = Vec::with_capacity(rx.len()); // we don't need packages from the previous iter, `recv_many` doesn't clear.

                // unfortunately `recv_many` blocks on empty channel and has clunky behaviour overall
                loop {
                    match rx.try_recv() {
                        Ok(packet) => recv_buf.push(packet),
                        Err(TryRecvError::Disconnected) => {
                            // only an error if program was not aborted
                            if abort_handler.abort() {
                                return Ok(());
                            } else {
                                bail!("channel closed")
                            }
                        }
                        Err(TryRecvError::Empty) => break,
                    }
                }
                let num_packets = recv_buf.len();
                let measurements = recv_buf.into_iter().into_group_map_by(|measurement| measurement.time.date_naive());

                for (idx, (date, mut packets)) in measurements.into_iter().enumerate() {
                    debug_assert!(packets.iter().all(|m| m.time.date_naive() == date));

                    trace!("try_recv(): Ok(packet) => process…");
                    if idx == 0 {
                        if let Some(packet) = packets.first() {
                            Span::current().record("when_first", format!("{:?}", packet.time));
                        }
                    }
                    if idx == num_packets {
                        if let Some(packet) = packets.last() {
                            Span::current().record("when_last", format!("{:?}", packet.time));
                        }
                    }

                    let basename = format!("{date}", date = date.format("%Y-%m-%d"));

                    // if file exists but some parsing/reading error occurs, append digit and try again.
                    let (save_file, mut all_objects) = handle_corrupted_json::<Vec<Measurement>>(&path, &basename, SAVE_FILE_EXT).await;
                    Span::current().record("target_file", &save_file.to_string_lossy().into_owned());

                    all_objects.append(&mut packets);

                    let writer = File::create(basename).await.context("opening DataObject JSON (2nd time, for writing)")?;
                    let mut writer = BrotliEncoder::with_quality(BufWriter::new(writer), async_compression::Level::Precise(BROTLI_LEVEL));
                    writer
                        .write_all(&serde_json::to_vec_pretty(&all_objects)?)
                        .await
                        .context("writing (updated) DataObject JSON")?;
                    // TODO (maybe) tokio fs is _the horror_, BufWrite drops remaining bytes on drop. build wrapper to ensure these shutdown instructions are always honed.
                    writer.flush().await?;
                    writer.shutdown().await?;
                }

                if num_packets == 0 {
                    debug!("no packets received, sleeping for 2min");
                } else {
                    debug!("successfully updated DataObjects, sleeping for 2min");
                }
                sleep(Duration::from_secs(120)).await;
                Ok(())
            }
            .instrument(debug_span!(
                "save_worker inner loop",
                num_packets = field::Empty,
                when_first = field::Empty,
                when_last = field::Empty,
                target_file = field::Empty
            ))
            .await?
        }
        Ok(())
    });

    Ok((handle, tx))
}

#[tracing::instrument]
async fn handle_corrupted_json<DeserT: Default + for<'a> Deserialize<'a>>(dir: &Path, basename: &str, ext: &str) -> (PathBuf, DeserT) {
    // TODO (unlikely) pull out try_reading() as closure parameter. Then this would be truly generic.
    async fn try_reading<DeserT: for<'a> Deserialize<'a>>(filename: &Path) -> Result<DeserT> {
        let desert_type = std::any::type_name::<DeserT>();

        let mut brotli = async_compression::tokio::bufread::BrotliDecoder::new(BufReader::new(File::open(filename).await?));
        let mut buf = String::new();
        brotli
            .read_to_string(&mut buf)
            .await
            .wrap_err_with(|| format!("reading {desert_type} JSON file"))?;

        Ok(serde_json::from_str::<DeserT>(&buf).wrap_err_with(|| format!("parsing {desert_type} JSON (from file)"))?)
    }

    let desert_type = std::any::type_name::<DeserT>();

    let mut counter: u16 = 0;
    let mut json_path = dir.join(format!("{basename}.{ext}"));

    loop {
        if !json_path.exists() {
            return (json_path, DeserT::default());
        }

        match try_reading(&json_path).await {
            Ok(json) => return (json_path, json),
            Err(e) => {
                error!("Attempt #{counter} at reading `{desert_type}` JSON … failed! {e:#}");
            }
        }

        // gen path befor inc counter, so we can have base.ext => base.0.ext => base.1.ext => …
        json_path = dir.join(format!("{basename}.{counter}.{ext}"));
        counter = counter.checked_add(1).unwrap_or_else(|| {
            panic!(
                "u16 overflow on `{json_path:?}` (appearently I've tried {} times :surprised_pikachu:)",
                u16::MAX
            )
        });
    }
}

#[tracing::instrument]
async fn update_last_recv(client_addr: SocketAddr, client_map: &mut ClientMap) -> Result<()> {
    let mut client_map = client_map.lock().await;
    client_map.entry(client_addr).or_default().update_last_recv();

    Ok(())
}

// TODO maybe implement `Abortable`
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

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;
    use brotli;
    use std::fs::OpenOptions;
    use std::io::{self, Write};
    use tempfile::tempdir;

    fn create_mock_json_br(file_path: &str, content: &str) -> io::Result<()> {
        let file = OpenOptions::new().write(true).create(true).truncate(true).open(file_path)?;
        let mut encoder = brotli::CompressorWriter::new(file, 4096, 5, 22);
        encoder.write_all(content.as_bytes())
    }

    #[tokio::test]
    async fn handle_corrupted_json__once() -> Result<()> {
        let temp_dir = tempdir()?;
        let (dir, base, ext) = (temp_dir.path(), "2024-12-01", "json.br");
        let corrupted_content = "{ invalid_json: ,}";

        // Create a corrupted JSON file
        create_mock_json_br(&format!("{dir}/{base}.{ext}", dir = dir.to_str().unwrap()), corrupted_content)?;

        // Simulate reading failure by calling the function
        let (result, _) = handle_corrupted_json::<()>(dir, base, ext).await;

        // Check if the new file was created with suffix .0.json.br
        let new_file_path = temp_dir.path().join("2024-12-01.0.json.br");
        assert_eq!(new_file_path, result);
        Ok(())
    }

    #[tokio::test]
    async fn handle_corrupted_json__multiple() -> Result<()> {
        let temp_dir = tempdir()?;
        let (dir, base, ext) = (temp_dir.path(), "2024-12-02", "json.br");
        let corrupted_content = "{ invalid_json: ,}";

        // Simulate multiple failures, incrementing suffix each time
        create_mock_json_br(&format!("{dir}/{base}.{ext}", dir = dir.to_str().unwrap()), corrupted_content)?;
        create_mock_json_br(&format!("{dir}/{base}.0.{ext}", dir = dir.to_str().unwrap()), corrupted_content)?;
        create_mock_json_br(&format!("{dir}/{base}.1.{ext}", dir = dir.to_str().unwrap()), corrupted_content)?;

        // handle_corrupted_json() should detect and skip every corrupt file
        let (result, _) = handle_corrupted_json::<()>(dir, base, ext).await;

        // Check if the second corrupted file was renamed to .2.json.br
        let new_file_path = temp_dir.path().join("2024-12-02.2.json.br");
        assert_eq!(new_file_path, result);
        Ok(())
    }

    #[tokio::test]
    async fn handle_corrupted_json__no_corruption() -> Result<()> {
        #[derive(Debug, Deserialize, PartialEq)]
        struct ValidContent {
            pub valid_json: bool,
        }
        let temp_dir = tempdir()?;
        let (dir, base, ext) = (temp_dir.path(), "2024-12-03", "json.br");
        let file_path_str = format!("{dir}/{base}.{ext}", dir = dir.to_str().unwrap());
        let valid_content = "[{ \"valid_json\": true }]";

        // Create a valid JSON file
        create_mock_json_br(&file_path_str, &valid_content)?;

        // Simulate reading success, should not trigger renaming
        let (result, parsed_data) = handle_corrupted_json::<Vec<ValidContent>>(dir, base, ext).await;

        // Ensure the original file still exists
        assert!(Path::new(&file_path_str).exists());
        assert_eq!(Path::new(&file_path_str), result);
        assert_eq!(parsed_data, vec![ValidContent { valid_json: true }]);
        Ok(())
    }

    #[tokio::test]
    async fn handle_corrupted_json__mixed_files() -> Result<()> {
        #[derive(Debug, Deserialize, PartialEq)]
        struct ValidContent {
            pub valid_json: bool,
        }

        let temp_dir = tempdir()?;
        let (dir, base, ext) = (temp_dir.path(), "2024-12-04", "json.br");
        let corrupted_content = "{ invalid_json: ,}";
        let valid_content = r#"[{ "valid_json": true }]"#;

        // Create a sequence of corrupted JSON files
        create_mock_json_br(&format!("{dir}/{base}.{ext}", dir = dir.to_str().unwrap()), corrupted_content)?;
        create_mock_json_br(&format!("{dir}/{base}.0.{ext}", dir = dir.to_str().unwrap()), corrupted_content)?;

        // Create a valid JSON file at the third iteration
        create_mock_json_br(&format!("{dir}/{base}.1.{ext}", dir = dir.to_str().unwrap()), valid_content)?;

        // Call the function
        let (result, parsed_data) = handle_corrupted_json::<Vec<ValidContent>>(dir, base, ext).await;

        // The function should return the valid file path
        let expected_file_path = temp_dir.path().join(format!("{base}.1.{ext}"));
        assert_eq!(expected_file_path, result);
        assert!(expected_file_path.exists());

        // The parsed content should match the valid JSON structure
        assert_eq!(parsed_data, vec![ValidContent { valid_json: true }]);
        Ok(())
    }

    #[tokio::test]
    #[should_panic(expected = "u16 overflow")]
    async fn handle_corrupted_json__u16_max_panic_SLOW() {
        let temp_dir = tempdir().unwrap();
        let (dir, base, ext) = (temp_dir.path(), "2024-12-05", "json.br");
        let corrupted_content = "{ invalid_json: ,}";

        // base JSON
        create_mock_json_br(dir.join(format!("{base}.{ext}")).to_str().unwrap(), corrupted_content).unwrap();
        // Create u16::MAX corrupted JSON files
        for i in 0..=u16::MAX {
            let base = format!("{base}.{i}.{ext}");
            let file_path = dir.join(base);
            create_mock_json_br(file_path.to_str().unwrap(), corrupted_content).unwrap();
        }

        // This should panic due to u16 overflow
        let _ = handle_corrupted_json::<()>(dir, base, ext).await;
    }
}

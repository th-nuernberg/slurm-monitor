use std::io::Write;
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::thread;
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Local;
use log::{debug, info, LevelFilter};
use nvml_wrapper::Nvml;

use sysinfo::{System, SystemExt};

mod config;

use collector_data::monitoring_info::State;
use config::Settings;

fn main() -> Result<()> {
    init_logger();

    // Load configuration
    let Settings {
        server_addr: server_ip,
        server_port,
        tx_interval,
    } = read_config()?;
    let tx_interval = tx_interval.unwrap();
    let server_addr = SocketAddr::new(server_ip, server_port);

    let mut sys = System::new_all();
    let nvml = Nvml::init()?;

    send_initial_data(server_addr, &mut sys, &nvml)?;

    loop {
        let pre = Instant::now();
        send_monitoring_data(server_addr, &mut sys, &nvml)?;

        let it_took = Instant::now().saturating_duration_since(pre);
        let sleep_for = tx_interval.to_std()?.saturating_sub(it_took);
        info!("collecting data took: {it_took:?}\nsleeping for {sleep_for:?}…");
        thread::sleep(sleep_for);
    }
}

fn init_logger() {
    env_logger::Builder::new()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .filter(None, LevelFilter::Info)
        .init();
}

fn read_config() -> Result<Settings> {
    info!("Loading config");
    Settings::new().map_err(anyhow::Error::new).context("parsing config file")
}

fn send_initial_data(server_addr: SocketAddr, sys: &mut System, nvml: &Nvml) -> Result<()> {
    // Send static information
    info!("Getting starting info");
    let starting_info = State::get_starting_info(sys, nvml).map_err(|e| anyhow::Error::msg(e.to_string()))?;
    debug!("Starting info: {starting_info}");

    log::info!("Connecting to server");
    let mut stream = TcpStream::connect(server_addr)?;
    let msg: &[u8] = starting_info.as_bytes();
    info!("Sending starting info");
    stream.write_all(msg)?;
    stream.flush()?;
    stream.shutdown(Shutdown::Both)?;
    Ok(())
}

fn send_monitoring_data(server_addr: SocketAddr, sys: &mut System, nvml: &Nvml) -> Result<()> {
    info!("Getting monitoring data");
    let monitoring_data = State::get_monitoring_data(sys, nvml).map_err(|e| anyhow::Error::msg(e.to_string()))?;
    debug!("Monitoring data: {monitoring_data}");

    let mut stream = TcpStream::connect(server_addr)?;
    log::info!("Cnode connected to server.");
    let msg: &[u8] = monitoring_data.as_bytes();
    stream.write_all(msg)?;
    stream.flush()?;
    stream.shutdown(Shutdown::Both)?;

    Ok(())
}

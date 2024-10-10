use std::{net::{IpAddr, SocketAddr, SocketAddrV4}, path::PathBuf};

use clap::Parser;
use tracing::Level;

#[derive(Debug, Clone, PartialEq, Parser)]
pub struct Args {
    pub log_level: Option<Level>,

    #[arg(short, long)]
    pub data_dir: PathBuf,

    #[arg(short, long, default_value = "0.0.0.0")]
    pub ip: IpAddr,
    #[arg(short, long, default_value = "19912")]
    pub port: u16,
}

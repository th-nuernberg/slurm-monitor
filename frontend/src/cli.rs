use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, PartialEq, Parser)]
pub struct Args {
    #[arg(long)]
    pub data_dir: PathBuf,
}

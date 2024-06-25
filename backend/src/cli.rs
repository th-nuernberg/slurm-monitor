use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, PartialEq, Parser)]
pub struct Args {
    pub data_dir: PathBuf,
}

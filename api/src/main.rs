use std::{
    collections::HashMap,
    future::IntoFuture,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{Date, DateTime, FixedOffset, NaiveDate, Utc};
use clap::Parser;
use collector_data::monitoring_info::Measurement;
use color_eyre::{
    eyre::{bail, ensure, eyre, Context},
    Report, Result, Section,
};
use derive_getters::Getters;
use futures::{FutureExt as _, TryFutureExt as _};
use poem::{listener::TcpListener, FromRequest, Route, Server};
use poem_openapi::{
    param::{Header, Query},
    payload::PlainText,
    types::{ParseFromJSON, ParseFromParameter, Type},
    NewType, OpenApi, OpenApiService,
};
use serde::{Deserialize, Serialize};
use tokio::{join, sync::RwLock};
use tracing::{error, info};

const SERVER_ADDR: &str = "localhost:3034";

type Data = Arc<RwLock<HashMap<NaiveDate, Arc<Vec<Measurement>>>>>;

struct Api {
    data: Data,
}

#[OpenApi]
impl Api {
    #[tracing::instrument(skip_all, fields(name = name.0))]
    #[oai(path = "/", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("Hello, {name}!")),
            None => PlainText(format!("Hello!")),
        }
    }

    #[oai(path = "/all", method = "post")]
    async fn all(&self, start: Header<Option<DateTime<Utc>>>, end: Header<Option<DateTime<Utc>>>) {}
}

#[derive(Debug, Clone, clap::Parser)]
pub struct Args {
    #[arg(long)]
    pub data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let data = Data::default();
    let data_future = load_data(data.clone(), &args.data_dir).map_ok(|e| {
        for err in e {
            error!("reading data file: {err:#}");
        }
    });

    let api_service = OpenApiService::new(Api { data: data.clone() }, "Hello World", "1.0").server(format!("http://{SERVER_ADDR}"));
    let ui = api_service.swagger_ui();
    let app = Route::new().nest("/", api_service).nest("/docs", ui);

    let server = Server::new(TcpListener::bind(SERVER_ADDR));
    let (server_result, data_result) = join!(server.run(app), data_future);
    // TODO (maybe) is there any way to join! Results so I don't have to unpack/log them all one by one?
    data_result?;
    server_result?;
    Ok(())
}

async fn load_datafile(data: Data, file: &Path) -> Result<NaiveDate> {
    // because there shouldn't be unrecognizable files inside `data_dir`
    let date = collector_data::parse_filename(&file).with_context(|| {
        format!(
            "checking if {file}'s name is in datafile format (collector_data::parse…) ",
            file = file.to_string_lossy()
        )
    })?;
    let measurements: Vec<Measurement> = serde_json::from_str(tokio::fs::read_to_string(&file).await.wrap_err_with(|| format!("reading {file:?}"))?.as_str())
        .wrap_err_with(|| format!("parsing {file:?}"))?;
    data.write().await.insert(date, measurements.into());

    Ok(date)
}

#[tracing::instrument]
async fn load_data(data: Data, data_dir: &Path) -> Result<Vec<Report>> {
    ensure!(data_dir.is_dir());

    // TODO (mabye) return iterator, for composability/laziness
    let n = data_dir.read_dir().context("counting data files")?.count();
    let mut errors: Vec<Report> = Vec::new();
    // TODO can parallelize await with Vec or sth probably. Or `spawn()` everything and join afterwards.
    for (idx, file) in data_dir.read_dir()?.enumerate() {
        let file = file.context("listing files in {data_dir:?}")?.path();
        match load_datafile(data.clone(), &file).await {
            Ok(date) => {
                info!(?file, ?date, "Data file successfully read: {idx}/{n}");
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    Ok(errors)
}

// symmetric over x==y
pub fn index_symmetric_matrix(x: usize, y: usize, total_size: usize) -> Result<usize> {
    // euler?
    fn square_number(n: usize) -> usize {
        n * (n + 1) / 2
    }
    ensure!(x < total_size && y < total_size);

    // y is the smaller part, y
    let (x, y) = (x.max(y), x.min(y));
    assert!(y <= x);

    let row_offset = square_number(x);

    Ok(row_offset + y)
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn random__index_symmetric_matrix() -> Result<()> {
        const N: usize = 10;
        for x in 0..N {
            for y in 0..N {
                print!(
                    "[{:width$}]",
                    index_symmetric_matrix(x, y, N)?,
                    width = ((N + 1) as f64).powi(2).log10().floor() as usize
                )
            }
            println!()
        }
        Ok(())
    }
}
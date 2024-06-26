mod cli;
mod parse;

use std::{fmt::format, fs::File, io::Read, ops::Deref, path::Path};

use anyhow::{anyhow, bail, Context, Result};
use axum::{
    handler::Handler,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_macros::debug_handler;
use clap::Parser;
use itertools::Itertools as _;
use maud::{html, Markup};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

static DATA_SACCT: Lazy<RwLock<Vec<String>>> = Lazy::new(|| RwLock::new(vec![]));

#[tokio::main]
async fn main() -> Result<()> {
    // initialize tracing
    // tracing_subscriber::fmt::init();

    let args = cli::Args::parse();

    let sacct_data_local = load_sacct(&args.data_dir)?;
    let (data, errors): (Vec<_>, Vec<_>) = sacct_data_local.into_iter().partition_result();
    errors
        .into_iter()
        .for_each(|e| eprintln!("{}", e.context("parsing sacct data")));
    *DATA_SACCT.deref().write().await = data;

    // build our application with a route
    /*let x = || async {
        match index(DATA_SACCT.blocking_read().deref()).await {
            Ok(response) => Response::builder().body(response),
            Err(error) => Response::builder().status(500).body(html! {
                h2 style="color=red;" { (format!("[ERROR]: error")) }
            }),
        }
    };*/
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(index));

    // run our app with hyper, listening globally on port 3333
    let port: u16 = 3333;
    let address = format!("0.0.0.0:{port}");

    let listener = tokio::net::TcpListener::bind(&address).await.unwrap();
    println!("Serving http://{address}");

    axum::serve(listener, app).await.unwrap();
    Ok(())
}

fn load_sacct(data_dir: impl AsRef<Path>) -> Result<Vec<Result<String>>> {
    let readdir = std::fs::read_dir(data_dir)?;
    let data = readdir
        .map(|entry| -> Result<Option<String>> {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!("{e}");
                    return Ok(None);
                }
            };

            if !entry.file_name().to_string_lossy().ends_with(".csv") {
                return Ok(None);
            }

            let error_context =
                || format!("reading entry {}", entry.file_name().to_str().unwrap_or(""));

            let filetype = entry.file_type().with_context(error_context)?;
            if !filetype.is_file() {
                return Result::Ok(None);
            }
            let mut file = File::open(entry.path()).with_context(error_context)?;
            let mut buf = String::new(); // TODO with_capacity(file_size)
            let _num_bytes = file.read_to_string(&mut buf).with_context(error_context)?;
            Result::Ok(Some(buf))
        })
        // Result<Option<Entry>> => Option<Result<Entry>> =(filter)=> Result<Entry>
        // since we don't care about None's
        .filter_map(|entry| match entry {
            Ok(Some(entry)) => Some(Ok(entry)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        })
        .collect_vec();

    Ok(data)
}

// (from: https://github.com/tokio-rs/axum/blob/main/examples/anyhow-error-response/src/main.rs)
// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[debug_handler]
// basic handler that responds with a static string
async fn index() -> Result<Markup, AppError> {
    // TODO update instead of taking only last

    Ok(html! {
        h1 { "Working!" }
        h2 { "Here be monitors…" }
        //p { "DEBUG" (format!("{:?}", data.clone().map(|d| d.map(|d| &d["jobs"]))))}
        @match sacct_table().await {
            Ok(data) => (data),
            Err(e) => h3 style="color: red" { (e) },
        }
    })
}

async fn sacct_table() -> Result<Markup> {
    let _table_fields = [
        "jobid",
        "jobidraw",
        "jobname",
        "account",
        "user",
        "elapsed",
        "state",
        "partition",
        "ntasks",
        "alloccpus",
        "reqmem",
        "maxrss",
        "averss",
        "avecpu",
        "consumedenergy",
    ];

    let data = DATA_SACCT.read().await;
    let Some(data) = data.last() else {
        return Err(anyhow!("Somehow global DATA_SACCT seems to be empty").into());
    };
    let (header, data) = match parse::sacct_csvlike(data) {
        Ok(data) => data,
        Err(_) => todo!(),
    };

    Ok(html! {
        table {
            thead {
                @for key in header.iter() {
                    th { (key) }
                }
            }
            tbody {
                @for line in data.iter().filter(|job| job.as_ref().is_ok_and(|job| job.get("State").is_some_and(|state| state == "RUNNING"))) {
                    @match line {
                        Ok(line) => tr {
                            @for key in header.iter() {
                                @match &line.get(key) {
                                    Some(val) => td { (val) },
                                    None => td style="color: red" { "ERROR" }
                                }
                            }
                        },
                        Err(e) => tr colspan=(header.len()) style="color: red" { (e) }
                    }
                }
            }
        }
    })
}

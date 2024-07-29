mod cli;
mod parse;
mod render;

use std::{
    collections::HashMap,
    fmt::format,
    fs::File,
    io::{Cursor, Read},
    ops::Deref,
    path::{Path, PathBuf},
    result::Result as StdResult,
};

use anyhow::{anyhow, bail, ensure, Context, Result};
use axum::{
    handler::Handler,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_macros::debug_handler;
use base64ct::{Base64, Encoding as _};
use clap::Parser;
use image::{io::Reader, ImageFormat, RgbImage};
use itertools::Itertools as _;
use log::error;
use maud::{html, Markup};
use once_cell::sync::Lazy;
use plotters::{
    backend::{BitMapBackend, DrawingBackend, SVGBackend},
    chart::ChartContext,
    coord::CoordTranslate,
};
use render::plot::simple_plot;
use tempfile::{spooled_tempfile, tempfile};
use tokio::sync::RwLock;

const SACCT_HEADER_JOBID: &str = "JobID";

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
async fn index() -> Markup {
    // TODO update instead of taking only last

    let jobcount_chart = make_jobcount_chart();

    html! {
        h1 { "Working!" }
        h2 { "Here be monitors…" }
        @match jobcount_chart.await {
            StdResult::Ok(data) => img src=(format!("data:image/png;base64,{data}", data=Base64::encode_string(&data))) {},
            Err(e) => h3 style="color: red" { (e) },
        }

        //p { "DEBUG" (format!("{:?}", data.clone().map(|d| d.map(|d| &d["jobs"]))))}
        @match sacct_table().await {
            StdResult::Ok(data) => (data),
            Err(e) => h3 style="color: red" { (e) },
        }
    }
}

fn make_chart<DB, CT>(title: impl AsRef<str>, chart: ChartContext<'_, DB, CT>) -> Result<Vec<u8>>
where
    DB: DrawingBackend,
    CT: CoordTranslate,
{
    let (x, y) = (800u32, 600u32);
    let mut buf = vec![0; (x * y * 3).try_into().unwrap()]; // RGB: bit depth = 24
    render::plot::simple_plot(
        BitMapBackend::with_buffer(buf.as_mut_slice(), (x, y)),
        title,
        dataset.iter().enumerate().map(|(x, &y)| (x)),
    )?;
    let mut image = RgbImage::from_raw(x, y, buf)
        .ok_or_else(|| anyhow!("failed to create image from internal buffer (too small?)"))
        .context("rendering job graph")?;

    let mut output_buf: Vec<u8> = Vec::new();
    image.write_to(&mut Cursor::new(&mut output_buf), ImageFormat::Png)?;

    Ok(output_buf)
}

async fn make_jobcount_chart() -> Result<Vec<u8>> {
    fn is_main_job(id: impl AsRef<str>) -> bool {
        !id.as_ref().contains('.')
    }

    fn get_id(job: &HashMap<String, String>) -> Result<String> {
        job.get(SACCT_HEADER_JOBID)
            .ok_or_else(|| {
                anyhow!("Data inconsistency at '{job:?}': `{SACCT_HEADER_JOBID}` not found")
            })
            .map(String::from)
    }
    let data = DATA_SACCT.read().await;
    if data.is_empty() {
        Err(anyhow!("no datasets found"))?;
    };

    let dataset = data
        .iter()
        .map(parse::sacct_csvlike)
        .map_ok(|(header, data)| {
            let jobid_key = header.iter().any(|s| *s == SACCT_HEADER_JOBID);
            if !jobid_key {
                bail!("Dataset contains no job ids!");
            };

            let job_ids = data
                .into_iter()
                .map(|j| j.and_then(|j| get_id(&j)))
                .filter(|j| j.is_err() || j.as_ref().is_ok_and(is_main_job)); // TODO why tf are map_ok and filter_ok not working ?!?
            let job_count = job_ids.process_results(|jobs| jobs.count());

            job_count
        })
        .flatten()
        .process_results(|x| x.collect_vec())?;

    let (x, y) = (800u32, 600u32);
    let mut buf = vec![0; (x * y * 3).try_into().unwrap()]; // RGB: bit depth = 24
    render::plot::simple_plot(
        BitMapBackend::with_buffer(buf.as_mut_slice(), (x, y)),
        "Number of active jobs",
        dataset
            .iter()
            .enumerate()
            .map(|(x, &y)| (x as f32, y as f32)),
    )?;
    let mut image = RgbImage::from_raw(x, y, buf)
        .ok_or_else(|| anyhow!("failed to create image from internal buffer (too small?)"))
        .context("rendering job graph")?;

    let mut output_buf: Vec<u8> = Vec::new();
    image.write_to(&mut Cursor::new(&mut output_buf), ImageFormat::Png)?;

    Ok(output_buf)
}

async fn make_memory_efficacy_chart() -> Result<Vec<u8>> {}

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
                        anyhow::Result::Ok(line) => tr {
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

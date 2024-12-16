pub mod data;

use std::{
    collections::HashMap,
    convert::Infallible,
    fmt::Display,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_compression::tokio::bufread::BrotliDecoder;
use chrono::{DateTime, NaiveDate, Utc};
use clap::Parser;
use collector_data::{gpu_dep::AllGpuTimesReportedBySlurm, monitoring_info::Measurement};
use color_eyre::{
    eyre::{ensure, Context},
    Report, Result,
};
use futures::TryFutureExt as _;
use itertools::Itertools as _;
use poem::{http::StatusCode, listener::TcpListener, Endpoint as _, EndpointExt, Route, Server};
use poem_openapi::{
    param::{Path as PathParam, Query},
    payload::{Json, PlainText},
    OpenApi, OpenApiService,
};
use serde::Serialize;
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
    join,
    sync::RwLock,
};
use tracing::{debug, error, info, level_filters::LevelFilter};

// TODO thread that periodically (30sec? on change?) reloads a data file if it is updated (may need locking)

const SERVER_ADDR: &str = "localhost:3034";
type Data = Arc<RwLock<Arc<HashMap<NaiveDate, Vec<Measurement>>>>>;

struct Api {
    data: Data,
}
#[OpenApi]
impl Api {
    // TODO limit currently applies _before_ filtering. That might be counterintuitive.
    fn err_into_500(error: impl std::fmt::Display) -> poem::Error {
        error!("Error generated during API call: {error:#}");
        poem::Error::from_string(format!("{error:#}"), StatusCode::INTERNAL_SERVER_ERROR)
    }

    async fn get_data(&self) -> Arc<HashMap<NaiveDate, Vec<Measurement>>> {
        let measurements: Arc<HashMap<_, Vec<_>>> = self.data.read().await.clone();
        measurements
    }
    fn filter_data<T>(
        data: &HashMap<T, Vec<Measurement>>,
        start: impl Deref<Target = Option<DateTime<Utc>>>,
        end: impl Deref<Target = Option<DateTime<Utc>>>,
        limit: impl Deref<Target = Option<usize>>,
    ) -> impl Iterator<Item = &Measurement> {
        data.values()
            .flatten()
            .filter(move |&measure| {
                start.unwrap_or(DateTime::<Utc>::MIN_UTC) <= measure.time && measure.time <= end.unwrap_or(DateTime::<Utc>::MAX_UTC)
            })
            .take(limit.unwrap_or(usize::MAX))
    }
    fn return_json<T, E>(data: Result<T, E>) -> Result<Json<serde_json::Value>, poem::Error>
    where
        T: Serialize,
        E: Display,
    {
        // TODO spawn_blocking would be better, but problem with 'static and internal refs inside data (I believe)
        let json = /*spawn_blocking(move ||*/ serde_json::to_value(&data.map_err(Self::err_into_500)?)/*)
            .await*/
            .map_err(Self::err_into_500)?/*?*/;

        Ok(Json(json))
    }

    //#[tracing::instrument(skip_all, fields(name = name.0))]
    #[oai(path = "/hello/:user", method = "get")]
    async fn index(&self, user: PathParam<Option<String>>) -> PlainText<String> {
        match user.0 {
            Some(name) => PlainText(format!("Hello, {name}!")),
            None => PlainText("Hello!".to_string()),
        }
    }

    /// Report total hours of GPU time that SLURM has allocated to users in the specified time range.
    ///
    /// Returns gpu hours per user, as a map `{"user1": 1.6666, "user2": 234.5}`. Hours are floats, calculated from the seconds that SLURM reports.
    /// `start` and `end` default to [`UNIX_EPOCH`] and [`now()`], respectively
    ///
    /// This queries `sreport` live for every query (a in-memory cache is planned).
    #[oai(path = "/gpu_hours/reserved", method = "get")]
    async fn gpu_hours_reserved(
        &self,
        start: Query<Option<DateTime<Utc>>>,
        end: Query<Option<DateTime<Utc>>>,
    ) -> Result<Json<serde_json::Value>, poem::Error> {
        // TODO cache result (1h ttl or so)
        let start = start.unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
        let end = end.unwrap_or_else(|| Utc::now());
        let hashmap: Result<_> = AllGpuTimesReportedBySlurm::query(start..end).map(|gpu_times| {
            HashMap::from(gpu_times)
                .into_iter()
                .map(|(user, duration)| (user, duration.num_seconds() as f64 / 3600f64))
                .collect::<HashMap<_, _>>()
        });

        Self::return_json(hashmap)
    }

    //user: PathParam<String>,

    #[oai(path = "/all", method = "get")]
    async fn all(
        &self,
        start: Query<Option<DateTime<Utc>>>,
        end: Query<Option<DateTime<Utc>>>,
        limit: Query<Option<usize>>,
    ) -> Result<Json<serde_json::Value>, poem::Error> {
        let data = self.get_data().await;

        Self::return_json::<_, Infallible>(Ok(Self::filter_data(&*data, start, end, limit).collect_vec()))
    }
}

#[derive(Debug, Clone, clap::Parser)]
pub struct Args {
    #[arg(long)]
    pub data_dir: PathBuf,
}

/*

#[derive(Object, Deserialize, Debug)]
struct CommonParams {
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    limit: Option<usize>,
}


pub struct WrapDataAccessEndpoint<E: Endpoint> {
    inner: E,
}

impl<E: Endpoint> Endpoint for WrapDataAccessEndpoint<E> {
    type Output = Response;

    fn call(&self, req: poem::Request) -> impl std::future::Future<Output = poem::Result<Self::Output>> + Send {
        async move {};
        std::future::ready(unimplemented!("moved to wrap_data_access_middleware"))
    }
}

async fn wrap_data_access_middleware<E: Endpoint>(inner: E, req: poem::Request) -> Result<impl IntoResponse, poem::Error> {
    //self.inner.get_response(req).await.data::<impl Fn() -> … or so>().unwrap()

    Ok(Response::builder().content_type("application/json").status(StatusCode::OK).body(
        json!({"hello": "world", "inner": inner.get_response(req).await.into_body().into_string().await.map_err(|e| format!("{e:#}"))}).to_string(),
    ))
}*/

/// `whorker_threads`: only workers for async tasks (tokio::spawn, main). spawn_blocking spawns extra threads
#[tokio::main(worker_threads = 4)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().with_max_level(LevelFilter::TRACE).init();
    let args = Args::parse();

    let data = Data::default();
    let data_future = load_data(data.clone(), &args.data_dir).map_ok(|e| {
        for err in e {
            error!("reading data file: {err:#}");
        }
    });

    let api_service = OpenApiService::new(Api { data: data.clone() }, "Hello World", "1.0").server(format!("http://{SERVER_ADDR}"));
    let ui = api_service.swagger_ui();
    let app = Route::new().nest("/", api_service).nest("/docs", ui).around(|route, request| async move {
        // request logging middleware
        debug!(?request, "received request");
        let response = route.call(request).await;
        response
    });

    let server = Server::new(TcpListener::bind(SERVER_ADDR));
    let (server_result, data_result) = join!(server.run(app), data_future);
    // TODO (maybe) is there any way to join! Results so I don't have to unpack/log them all one by one?
    data_result?;
    server_result?;
    Ok(())
}

#[tracing::instrument(skip(data))]
async fn load_datafile(data: Data, file: &Path) -> Result<NaiveDate> {
    // because there shouldn't be unrecognizable files inside `data_dir`
    let date = collector_data::parse_filename(file).with_context(|| {
        format!(
            "checking if {file}'s name is in datafile format (collector_data::parse…) ",
            file = file.to_string_lossy()
        )
    })?;
    let mut reader = BrotliDecoder::new(BufReader::new(File::open(&file).await?));
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.wrap_err_with(|| format!("reading {file:?}"))?;
    let input = String::from_utf8_lossy(&buf).into_owned();
    //trace!("buf=`{buf:?}`");
    let measurements: Vec<Measurement> = serde_json::from_str(&input).wrap_err_with(|| format!("parsing {file:?}"))?;

    // RwLock - keep lock for as short as possible
    {
        let mut write_lock = data.write().await;
        // extract a &mut Arc<_> from a RwLockWriteGuard<Arc<_>>
        let global_data: &mut Arc<_> = &mut *write_lock;
        // Arc is immutable, so clone hash table from inside RwLockGuard and Arc so that we can mutate (insert) the new file
        // (mem::take would be nice so we don't have to clone, but then we can't use Arc and let API functions hold the data while it not being locked. Would be a trade-off)
        let mut local_data = (**global_data).clone();
        local_data.insert(date, measurements);
        // then, swap our table with the global one
        *global_data = Arc::new(local_data)
        //&**x = Arc::new(local_hashtable)
    }

    Ok(date)
}

#[tracing::instrument(skip(data))]
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
    info!("finished loading.");

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

use cen::SignedReport;
use futures::TryFutureExt;
use once_cell::sync::Lazy;
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, EnvFilter};
use warp::Filter;
use structopt::StructOpt;
use tracing::{info};

mod error;
mod storage;
mod timestamp;

static STORAGE: Lazy<storage::Storage> = Lazy::new(storage::Storage::default);
static OPTIONS: Lazy<Opt> = Lazy::new(Opt::from_args);

pub use timestamp::ReportTimestamp;

#[derive(Debug, StructOpt)]
struct Opt {
    /// The time interval over which to batch reports, in seconds.
    #[structopt(short, long, default_value = "21600")]
    seconds_per_batch: u64,
    /// The socket address to bind to.
    #[structopt(short, long, default_value = "127.0.0.1:3030")]
    address: std::net::SocketAddr,
}

#[tokio::main]
async fn main() {
    color_backtrace::install();

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("trace"))
        .unwrap();

    tracing_subscriber::fmt()
        .with_target(false)
        .finish()
        .with(ErrorLayer::default())
        .with(filter)
        .init();

    info!(options = ?*OPTIONS);

    let storage = &*STORAGE;
    let submit = warp::path!("submit")
        .and(warp::filters::method::post())
        .and(warp::filters::body::content_length_limit(1024 * 2))
        .and(warp::filters::body::bytes())
        .and_then(move |body: bytes::Bytes| async move {
            let report = SignedReport::read(body.as_ref()).map_err(error::into_warp)?;
            storage
                .save(report)
                .map_err(|e| e.wrap_err("Failed to save report"))
                .map_err(error::into_warp)
                .await
        });

    let get = warp::path!("get_reports" / ReportTimestamp)
        .and(warp::filters::method::get())
        .and_then(move |timeframe| {
            storage
                .get(timeframe)
                .map_err(|e| e.wrap_err("Failed to retrieve reports"))
                .map_err(error::into_warp)
        });

    warp::serve(submit.or(get).recover(error::handle_rejection))
        .run(OPTIONS.address)
        .await;
}

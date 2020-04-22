use futures::TryFutureExt;
use once_cell::sync::Lazy;
use structopt::StructOpt;
use tcn::SignedReport;
use tracing::info;
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, EnvFilter};
use warp::Filter;

mod error;
mod shard;
mod storage;
mod timestamp;

static STORAGE: Lazy<storage::Storage> = Lazy::new(storage::Storage::default);
static OPTIONS: Lazy<Opt> = Lazy::new(Opt::from_args);

pub use shard::Shard;
pub use timestamp::ReportTimestamp;

#[derive(Debug, StructOpt)]
struct Opt {
    /// The time interval over which to batch reports, in seconds.
    ///
    /// The default value is 21600 = 6h.  This needs to be adjusted
    /// when using the server in simulation mode, e.g., to 6s.
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
        .with_env_filter(filter)
        .finish()
        .with(ErrorLayer::default())
        .init();

    info!(options = ?*OPTIONS);

    let storage = &*STORAGE;
    let submit = warp::path!("submit" / Shard)
        .and(warp::filters::method::post())
        .and(warp::filters::body::content_length_limit(1024 * 2))
        .and(warp::filters::body::bytes())
        .and_then(move |shard, body: bytes::Bytes| async move {
            let report = SignedReport::read(body.as_ref()).map_err(error::into_warp)?;
            storage
                .save(shard, report)
                .map_err(|e| e.wrap_err("Failed to save report"))
                .map_err(error::into_warp)
                .await
        });

    let get = warp::path!("get_reports" / Shard / ReportTimestamp)
        .and(warp::filters::method::get())
        .and_then(move |shard, timeframe| {
            storage
                .get(shard, timeframe)
                .map_err(|e| e.wrap_err("Failed to retrieve reports"))
                .map_err(error::into_warp)
        });

    warp::serve(submit.or(get).recover(error::handle_rejection))
        .run(OPTIONS.address)
        .await;
}

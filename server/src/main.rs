use cen::SignedReport;
use futures::TryFutureExt;
use once_cell::sync::Lazy;
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, EnvFilter};
use warp::reject::Rejection;
use warp::Filter;

pub use timestamp::ReportTimestamp;

mod error;
mod storage;
mod timestamp;

static STORAGE: Lazy<storage::Storage> = Lazy::new(storage::Storage::default);

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

    let storage = &*STORAGE;
    let submit = warp::path!("submit")
        .and(warp::filters::method::post())
        .and(warp::filters::body::content_length_limit(1024 * 2))
        .and(warp::filters::body::bytes())
        .and_then(move |body: bytes::Bytes| async move {
            let report = SignedReport::read(body.as_ref())?;
            storage
                .save(report)
                .map_err(|e| e.wrap_err("Failed to save report"))
                .map_err(Rejection::from)
                .await
        });

    let get = warp::path!("get_reports" / ReportTimestamp)
        .and(warp::filters::method::get())
        .and_then(move |timeframe| {
            storage
                .get(timeframe)
                .map_err(|e| e.wrap_err("Failed to retrieve reports"))
                .map_err(Rejection::from)
        });

    warp::serve(submit.or(get).recover(error::handle_rejection))
        .run(([127, 0, 0, 1], 3030))
        .await;
}

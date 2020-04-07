use cen::SignedReport;
use warp::Filter;
use std::sync::{Arc, Mutex};

pub use timestamp::ReportTimestamp;

mod storage;
mod timestamp;

#[tokio::main]
async fn main() {
    let mutex = Arc::new(Mutex::new(storage::Storage::default()));
    let mutex_ = mutex.clone();
    let submit = warp::path!("submit")
        .and(warp::filters::method::post())
        .and(warp::filters::body::content_length_limit(1024 * 2))
        .and(warp::filters::body::bytes())
        .map(|body: bytes::Bytes| SignedReport::read(body.as_ref()).unwrap())
        .map(move |report| {
            mutex_.lock().unwrap().save(report).unwrap();
            Ok(format!("report saved"))
        });

    let get = warp::path!("get_reports" / ReportTimestamp)
        .and(warp::filters::method::get())
        .map(move |timeframe| {
            let reports = mutex.lock().unwrap().get(timeframe).unwrap();
            reports
        });

    warp::serve(submit.or(get))
        .run(([127, 0, 0, 1], 3030))
        .await;
}

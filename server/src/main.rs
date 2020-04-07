use cen::SignedReport;
use warp::Filter;

pub use timestamp::ReportTimestamp;

mod storage;
mod timestamp;

#[tokio::main]
async fn main() {
    let submit = warp::path!("submit")
        .and(warp::filters::method::post())
        .and(warp::filters::body::content_length_limit(1024 * 2))
        .and(warp::filters::body::bytes())
        .map(|body: bytes::Bytes| SignedReport::read(body.as_ref()).unwrap())
        .map(|report| {
            storage::save(report).unwrap();
            Ok(format!("report saved"))
        });

    let get = warp::path!("get_reports" / ReportTimestamp)
        .and(warp::filters::method::get())
        .map(|timeframe| {
            let reports = storage::get(timeframe).unwrap();
            reports
        });

    warp::serve(submit.or(get))
        .run(([127, 0, 0, 1], 3030))
        .await;
}

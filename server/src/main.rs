use cen::SignedReport;
use warp::{Filter, Rejection};

pub use timestamp::ReportTimestamp;

mod error;
mod storage;
mod timestamp;

#[tokio::main]
async fn main() {
    let submit = warp::path!("submit")
        .and(warp::filters::method::post())
        .and(warp::filters::body::content_length_limit(1024 * 2))
        .and(body_filter());

    let get = warp::path!("get_reports" / ReportTimestamp)
        .and(warp::filters::method::get())
        .and_then(|timeframe| async move {
            let reports = storage::get(timeframe).map_err(error::into_warp)?;
            Ok::<_, Rejection>(reports)
        });

    warp::serve(submit.or(get).recover(error::handle_rejection))
        .run(([127, 0, 0, 1], 3030))
        .await;
}

fn body_filter() -> impl Filter<Extract = (String,), Error = Rejection> + Copy {
    warp::filters::body::bytes()
        .and_then(|body: bytes::Bytes| async move {
            SignedReport::read(body.as_ref()).map_err(error::into_warp)
        })
        .and_then(|report: SignedReport| async move {
            storage::save(report).map_err(error::into_warp)?;
            Ok::<_, Rejection>(format!("report saved"))
        })
}

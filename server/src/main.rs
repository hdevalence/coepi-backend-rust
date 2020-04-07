use warp::Filter;

mod storage;

struct CoepiReport(bytes::Bytes);
struct ReportTimeframe;

#[tokio::main]
async fn main() {
    let submit = warp::path!("submit")
        .and(warp::filters::method::post())
        .and(warp::filters::body::content_length_limit(1024 * 2))
        .and(warp::filters::body::bytes())
        .map(CoepiReport)
        .map(|report| {
            storage::save(report).unwrap();
            Ok(format!("report saved"))
        });

    warp::serve(submit).run(([127, 0, 0, 1], 3030)).await;
}

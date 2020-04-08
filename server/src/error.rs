use std::convert::Infallible;
use warp::reject::RejectionDebug;
use warp::{http::StatusCode, Rejection, Reply};

pub(crate) mod context;

pub(crate) type ErrReport = eyre::ErrReport<context::Context>;

pub(crate) async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message =
            format!("Error: {}\nNote: The supported endpoints by this server are `submit` and `get_reports/<timestamp>`\n", code);
    } else if let Some(report) = err.find::<ErrReport>() {
        code = report.context().status;
        message = format!("Error: {:?}\n", report);
    } else {
        // We should have expected this... Just log and say its a 500
        eprintln!("unhandled rejection: {:?}", err.debug());
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION\n".into();
    }

    Ok(warp::reply::with_status(message, code))
}

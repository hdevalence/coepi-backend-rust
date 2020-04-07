use serde::Serialize;
use std::convert::Infallible;
use warp::{http::StatusCode, Rejection, Reply};

pub(crate) mod context;

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

#[derive(Debug)]
pub(crate) struct ErrReport(eyre::ErrReport<context::Context>);

impl ErrReport {
    pub(crate) fn wrap_err<D>(self, msg: D) -> Self
    where
        D: std::fmt::Display + Send + Sync + 'static,
    {
        Self(self.0.wrap_err(msg))
    }
}

impl<E> From<E> for ErrReport
where
    E: Into<eyre::ErrReport<context::Context>>,
{
    fn from(inner: E) -> Self {
        Self(inner.into())
    }
}

impl warp::reject::Reject for ErrReport {}

impl std::fmt::Display for ErrReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

pub(crate) fn into_warp(report: impl Into<ErrReport>) -> Rejection {
    warp::reject::custom(report.into())
}

pub(crate) async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND".into();
    } else if let Some(report) = err.find::<ErrReport>() {
        code = StatusCode::BAD_REQUEST;
        message = format!("Error: {:?}", report);
    } else {
        // We should have expected this... Just log and say its a 500
        eprintln!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION".into();
    }

    let json = warp::reply::json(&ErrorMessage {
        code: code.as_u16(),
        message,
    });

    Ok(warp::reply::with_status(json, code))
}

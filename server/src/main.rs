use bytes::Bytes;
use deadpool_redis::Config;
use once_cell::sync::Lazy;
use serde::*;
use std::{collections::HashSet, ops::DerefMut};
use warp::Filter;

mod endpoints;
use endpoints::*;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

static POOL: Lazy<deadpool_redis::Pool> = Lazy::new(|| {
    let cfg = Config::from_env("REDIS").expect("need redis config");
    let pool = cfg.create_pool().expect("failed to create connection pool");
    pool
});

#[derive(Debug)]
struct BadJson;

impl warp::reject::Reject for BadJson {}

#[derive(Debug)]
struct QueryFailed;

impl warp::reject::Reject for QueryFailed {}

macro_rules! mk_handler {
    ($route: ident) => {
        warp::path(stringify!($route))
            .and(warp::body::bytes())
            .and_then(|body: Bytes| async move {
                let req: $route::Req =
                    serde_json::from_slice(&body).map_err(|_| warp::reject::custom(BadJson))?;
                let res = req
                    .handle()
                    .await
                    .map_err(|_| warp::reject::custom(QueryFailed))?;
                let json =
                    serde_json::to_string(&res).map_err(|_| warp::reject::custom(QueryFailed))?;
                Ok::<_, warp::reject::Rejection>(json)
            })
            .boxed()
    };
}

#[tokio::main]
async fn main() {
    let routes = mk_handler!(report_new_symptoms)
        .or(mk_handler!(get_symptoms))
        .or(mk_handler!(get_cases));

    warp::serve(routes).bind(([0u8, 0, 0, 0], 8080)).await;
}

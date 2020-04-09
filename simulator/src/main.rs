use eyre::ErrReport;
use once_cell::sync::Lazy;
use structopt::StructOpt;
use tracing::info;
use tracing_error::ErrorLayer;
use tracing_subscriber::{prelude::*, EnvFilter};

mod user;
use user::User;

#[derive(Debug, StructOpt)]
pub struct Opt {
    /// Global time scale factor.  All times marked (simtime) will be divided by this factor.
    #[structopt(long, default_value = "3600")]
    time_warp: f64,

    /// Server URL
    #[structopt(short = "s", long, default_value = "http://127.0.0.1:3030")]
    server: String,

    /// Server batch interval, in seconds (realtime).
    #[structopt(short = "t", long, default_value = "6")]
    server_batch_interval: u64,

    /// Contact Probability per TCK interval.
    #[structopt(long, default_value = "0.0001")]
    contact_probability: f64,

    /// TCK rotation interval, in seconds (simtime)
    #[structopt(long, default_value = "300")]
    tck_rotation_secs: u64,

    /// RAK rotation interval, in seconds (simtime)
    #[structopt(long, default_value = "86400")]
    rak_rotation_secs: u64,

    /// Number of days of history to report upon infection (simtime)
    #[structopt(long, default_value = "14")]
    incubation_period_days: u64,

    /// Number of users to simulate
    #[structopt(short = "n", long, default_value = "100")]
    num_users: usize,

    /// The probability that a user becomes infected in each rak interval.
    #[structopt(long, default_value = "0.01")]
    report_probability: f64,

    /// The number of days to run the simulation (simtime)
    #[structopt(long, default_value = "28")]
    simulation_days: u64,
}

static OPTIONS: Lazy<Opt> = Lazy::new(Opt::from_args);

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

    // check that the URL is parseable upfront
    // XXX parse in structopt
    reqwest::Url::parse(&OPTIONS.server).unwrap();

    info!(options = ?*OPTIONS);

    let tcn_broadcast_buffer_size = OPTIONS.num_users * 20;
    let (tx, _) = tokio::sync::broadcast::channel(tcn_broadcast_buffer_size);

    let mut users = futures::stream::FuturesUnordered::new();

    use std::time::Duration;
    use tokio::time::delay_for;

    for id in 0..OPTIONS.num_users {
        // Stagger the start of each user.
        delay_for(Duration::from_millis(1)).await;
        users.push(tokio::spawn(User::default().run(id, tx.clone())));
    }

    use futures::prelude::*;
    let results = users.collect::<Vec<_>>().await;
    info!(?results);
}

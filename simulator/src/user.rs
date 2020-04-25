use eyre::eyre;
use eyre::ErrReport;
use futures::prelude::*;
use rand::{
    distributions::{Bernoulli, Distribution, Uniform},
    thread_rng,
};
use std::collections::{BTreeSet, HashMap};
use std::convert::TryInto;
use std::io::Cursor;
use std::time::Duration;
use tcn::{ReportAuthorizationKey, SignedReport, TemporaryContactKey, TemporaryContactNumber};
use tokio::prelude::*;
use tokio::sync::broadcast;
use tokio::time::Instant;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::OPTIONS;

use crate::shard::Shard;
use crate::shard::ShardId;

pub struct User {
    rak: ReportAuthorizationKey, // Current rak
    rak_shards: Vec<ShardId>,    // Shards this rak was used in
    shard: Shard,                // Current shard
    shard_hist: Vec<ShardId>,    // Shards since last report fetch
    raks: Vec<(ReportAuthorizationKey, Vec<ShardId>)>,
    tck: TemporaryContactKey,
    observed_tcns: BTreeSet<TemporaryContactNumber>,
}

impl User {
    pub fn init(shard_id: ShardId, tx: broadcast::Sender<TemporaryContactNumber>) -> User {
        let rak = ReportAuthorizationKey::new(thread_rng());
        let tck = rak.initial_temporary_contact_key();
        User {
            rak,
            rak_shards: vec![shard_id],
            raks: Vec::new(),
            shard: Shard::init(shard_id, tx),
            shard_hist: vec![shard_id],
            tck,
            observed_tcns: BTreeSet::default(),
        }
    }
}

impl User {
    #[instrument(skip(self, channels))]
    pub async fn run(
        mut self,
        id: usize,
        channels: HashMap<ShardId, broadcast::Sender<TemporaryContactNumber>>,
    ) -> Result<(), ErrReport> {
        // We will use the tck rotation as the root time ticker.
        let warped_tck_rotation =
            Duration::from_secs(OPTIONS.tck_rotation_secs).div_f64(OPTIONS.time_warp);

        let tcks_per_rak: u16 = (OPTIONS.rak_rotation_secs / OPTIONS.tck_rotation_secs)
            .try_into()
            .unwrap();
        let max_tcks = 86400 * OPTIONS.simulation_days / OPTIONS.tck_rotation_secs;
        let tcn_observation = Bernoulli::new(OPTIONS.contact_probability).unwrap();
        let report_probability = Bernoulli::new(OPTIONS.report_probability).unwrap();
        let shard_choices = Uniform::new(0u64, OPTIONS.num_shards);
        let shard_change_probability = Bernoulli::new(OPTIONS.shard_change_probability).unwrap();
        let server_batch_interval = Duration::from_secs(OPTIONS.server_batch_interval);

        info!(
            id,
            ?warped_tck_rotation,
            tcks_per_rak,
            max_tcks,
            "launching task for user"
        );
        let mut interval_stream =
            tokio::time::interval(warped_tck_rotation).take(max_tcks as usize);

        let mut last_check = Instant::now();

        while let Some(time) = interval_stream.next().await {
            // Check whether we should rotate the rak, and if so, whether we should report.
            let should_report = if self.tck.index() > tcks_per_rak {
                self.rotate_rak();
                report_probability.sample(&mut thread_rng())
            } else {
                false
            };

            // Generate and broadcast a TCN.
            self.broadcast();

            // Listen for TCNs broadcast by others.
            self.observe(&tcn_observation);

            // Change to random shard sometimes
            self.change_shard(&channels, &shard_choices, &shard_change_probability);

            // Optionally fetch new reports from the server.
            if time > (last_check + server_batch_interval) {
                self.fetch_reports().await?;
                last_check = time;
            }

            if should_report {
                self.send_reports().await?;
            }
        }

        Ok(())
    }

    #[instrument(skip(self))]
    fn rotate_rak(&mut self) {
        debug!("rotating rak");
        self.raks.push((self.rak, self.rak_shards.clone()));
        self.rak = ReportAuthorizationKey::new(thread_rng());
        self.rak_shards = Vec::new();
        self.tck = self.rak.initial_temporary_contact_key();
    }

    #[instrument(skip(self))]
    fn broadcast(&mut self) {
        let tcn = self.tck.temporary_contact_number();
        self.tck = self.tck.ratchet().unwrap();
        trace!(?tcn);
        self.shard.tx.send(tcn).expect("broadcast should succeed");
    }

    #[instrument(skip(self))]
    fn change_shard(
        &mut self,
        channels: &HashMap<ShardId, broadcast::Sender<TemporaryContactNumber>>,
        shard_choices: &Uniform<ShardId>,
        shard_change_probability: &Bernoulli,
    ) {
        if shard_change_probability.sample(&mut thread_rng()) {
            self.rak_shards.push(self.shard.id);
            self.shard_hist.push(self.shard.id);
            let new_shard = shard_choices.sample(&mut thread_rng());
            let new_tx = channels.get(&new_shard).unwrap().clone();

            self.shard = Shard::init(new_shard, new_tx);
        }
    }

    #[instrument(skip(self, tcn_observation))]
    fn observe(&mut self, tcn_observation: &Bernoulli) -> Result<(), ErrReport> {
        loop {
            use broadcast::TryRecvError;
            match self.shard.rx.try_recv() {
                Ok(tcn) => {
                    if tcn_observation.sample(&mut thread_rng()) {
                        debug!(?tcn, "observed tcn broadcast");
                        self.observed_tcns.insert(tcn);
                    }
                }
                Err(TryRecvError::Lagged(skipped)) => {
                    warn!(skipped, "could not keep up with broadcasts");
                }
                Err(TryRecvError::Closed) => {
                    error!("broadcast channel closed");
                    return Err(eyre!("broadcast channel closed unexpectedly"));
                }
                Err(TryRecvError::Empty) => {
                    // Break out of the loop, no more broadcasts to process.
                    return Ok(());
                }
            }
        }
    }

    #[instrument(skip(self))]
    async fn fetch_reports(&mut self) -> Result<(), ErrReport> {
        use std::time::SystemTime;
        let batch_index = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs()
            / OPTIONS.server_batch_interval;

        for shard_id in self.shard_hist.iter() {
            let report_url = reqwest::Url::parse(&OPTIONS.server)?
	        // set shard_id as root
                .join(&(shard_id.to_string() + "/"))?
		.join("get_reports/")?
                // get previous batch
                .join(&(batch_index - 1).to_string())?;

            debug!(?report_url, "fetching reports");
            let rsp = reqwest::get(report_url).await?;

            match rsp.status() {
                reqwest::StatusCode::NOT_FOUND => {
                    debug!("Got 404 (empty record)");
                    return Ok(());
                }
                reqwest::StatusCode::OK => {
                    debug!("Got report data from server");
                }
                e => {
                    // can we attach rsp info here?
                    return Err(eyre!("got unknown status code {}", e));
                }
            }

            let bytes = rsp.bytes().await?;

            // Parse each report and process it
            tokio::task::block_in_place(|| {
                let mut candidate_tcns = BTreeSet::new();

                // Parse and expand reports
                let mut reader = Cursor::new(bytes.as_ref());
                while let Ok(signed_report) = SignedReport::read(&mut reader) {
                    match signed_report.verify() {
                        Ok(report) => {
                            candidate_tcns.extend(report.temporary_contact_numbers());
                        }
                        Err(_) => {
                            warn!("got report with invalid signature");
                        }
                    }
                }

                let mut matches = candidate_tcns.intersection(&self.observed_tcns);
                while let Some(tcn) = matches.next() {
                    info!(?tcn, ?shard_id, "got report about observed tcn from shard");
                }
            });
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn send_reports(&mut self) -> Result<(), ErrReport> {
        self.raks.push((self.rak, self.rak_shards.clone()));

        let raks_to_report =
            (86400 * OPTIONS.incubation_period_days / OPTIONS.rak_rotation_secs) as usize;

        info!(raks_to_report, "sending reports from most recent raks");

        let tcks_per_rak: u16 = (OPTIONS.rak_rotation_secs / OPTIONS.tck_rotation_secs)
            .try_into()
            .unwrap();


        let client = reqwest::Client::new();

        for (rak, shard_ids) in self.raks.iter().rev().take(raks_to_report) {
            let report = rak
                .create_report(tcn::MemoType::CoEpiV1, Vec::new(), 1, tcks_per_rak)
                .expect("memo data is not too long");

            let mut report_bytes = Vec::new();
            report
                .write(Cursor::new(&mut report_bytes))
                .expect("writing should succeed");

            for shard_id in shard_ids.iter() {
		let report_url = reqwest::Url::parse(&OPTIONS.server)?
		    .join(&(shard_id.to_string()+"/"))?
		    .join("submit/")?;

                debug!(shard_id, "sending report to shard");
                client
                    .post(report_url.clone())
                    .body(report_bytes.clone())
                    .send()
                    .await?;
            }
        }

        Ok(())
    }
}

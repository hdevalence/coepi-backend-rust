use eyre::eyre;
use eyre::ErrReport;
use futures::prelude::*;
use rand::{
    distributions::{Bernoulli, Distribution},
    thread_rng,
};
use std::collections::BTreeSet;
use std::convert::TryInto;
use std::io::Cursor;
use std::time::Duration;
use tcn::{ReportAuthorizationKey, SignedReport, TemporaryContactKey, TemporaryContactNumber};
use tokio::prelude::*;
use tokio::sync::broadcast;
use tokio::time::Instant;
use tracing::{debug, error, info, instrument, trace, warn};

use crate::OPTIONS;

pub struct User {
    raks: Vec<ReportAuthorizationKey>,
    tck: TemporaryContactKey,
    observed_tcns: BTreeSet<TemporaryContactNumber>,
}

impl Default for User {
    fn default() -> User {
        let rak = ReportAuthorizationKey::new(thread_rng());
        let tck = rak.initial_temporary_contact_key();
        User {
            raks: vec![rak],
            tck,
            observed_tcns: BTreeSet::default(),
        }
    }
}

impl User {
    #[instrument(skip(self, tx))]
    pub async fn run(
        mut self,
        id: usize,
        mut tx: broadcast::Sender<TemporaryContactNumber>,
    ) -> Result<(), ErrReport> {
        let mut rx = tx.subscribe();

        // We will use the tck rotation as the root time ticker.
        let warped_tck_rotation =
            Duration::from_secs(OPTIONS.tck_rotation_secs).div_f64(OPTIONS.time_warp);

        let tcks_per_rak: u16 = (OPTIONS.rak_rotation_secs / OPTIONS.tck_rotation_secs)
            .try_into()
            .unwrap();
        let max_tcks = 86400 * OPTIONS.simulation_days / OPTIONS.tck_rotation_secs;
        let mut tcn_observation = Bernoulli::new(OPTIONS.contact_probability).unwrap();
        let report_probability = Bernoulli::new(OPTIONS.report_probability).unwrap();
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
            self.broadcast(&mut tx);

            // Listen for TCNs broadcast by others.
            self.observe(&mut rx, &mut tcn_observation);

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
        let rak = ReportAuthorizationKey::new(thread_rng());
        self.tck = rak.initial_temporary_contact_key();
        self.raks.push(rak);
    }

    #[instrument(skip(self, tx))]
    fn broadcast(&mut self, tx: &mut broadcast::Sender<TemporaryContactNumber>) {
        let tcn = self.tck.temporary_contact_number();
        self.tck = self.tck.ratchet().unwrap();
        trace!(?tcn);
        tx.send(tcn).expect("broadcast should succeed");
    }

    #[instrument(skip(self, rx, tcn_observation))]
    fn observe(
        &mut self,
        rx: &mut broadcast::Receiver<TemporaryContactNumber>,
        tcn_observation: &mut Bernoulli,
    ) -> Result<(), ErrReport> {
        loop {
            use broadcast::TryRecvError;
            match rx.try_recv() {
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

        let report_url = reqwest::Url::parse(&OPTIONS.server)?
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
                info!(?tcn, "got report about observed tcn");
            }
        });

        Ok(())
    }

    #[instrument(skip(self))]
    async fn send_reports(&mut self) -> Result<(), ErrReport> {
        let raks_to_report =
            (86400 * OPTIONS.incubation_period_days / OPTIONS.rak_rotation_secs) as usize;

        info!(raks_to_report, "sending reports from most recent raks");

        let tcks_per_rak: u16 = (OPTIONS.rak_rotation_secs / OPTIONS.tck_rotation_secs)
            .try_into()
            .unwrap();

        let report_url = reqwest::Url::parse(&OPTIONS.server)?.join("submit/")?;

        let client = reqwest::Client::new();

        for rak in self.raks.iter().rev().take(raks_to_report) {
            let report = rak
                .create_report(tcn::MemoType::CoEpiV1, Vec::new(), 1, tcks_per_rak)
                .expect("memo data is not too long");

            let mut report_bytes = Vec::new();
            report
                .write(Cursor::new(&mut report_bytes))
                .expect("writing should succeed");

            client
                .post(report_url.clone())
                .body(report_bytes)
                .send()
                .await?;
        }

        Ok(())
    }
}

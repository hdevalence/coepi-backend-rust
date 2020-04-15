use super::{error::ErrReport, ReportTimestamp, SignedReport, Shard};
use crate::error::context::Status;
use eyre::eyre;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::{debug, info, instrument};
use warp::http::StatusCode;

pub(crate) enum StorageEntry {
    /// The storage entry is accepting new reports.
    Open(Vec<SignedReport>),
    /// The storage entry is finalized, and contains a serialization of
    /// all reports for the time interval in random order.
    Sealed(Vec<u8>),
}

impl Default for StorageEntry {
    fn default() -> Self {
        StorageEntry::Open(Vec::default())
    }
}

impl StorageEntry {
    /// Seal the entry, if it is open.
    fn seal(&mut self) {
        *self = match self {
            StorageEntry::Sealed(_) => return,
            StorageEntry::Open(ref mut reports) => {
                let count = reports.len();
                // Shuffle reports before serializing them.
                reports.shuffle(&mut OsRng);
                let mut bytes = Vec::<u8>::new();
                for report in reports {
                    report
                        .write(&mut bytes)
                        .expect("report serialization should be infallible");
                }
                info!(
                    count,
                    num_bytes = bytes.len(),
                    "sealed reports into byte buffer"
                );
                StorageEntry::Sealed(bytes)
            }
        }
    }
}

#[derive(Default)]
pub struct Storage {
    map: Mutex<HashMap<Shard, HashMap<ReportTimestamp, StorageEntry>>>,
}

// TODO: Add shard
impl Storage {
    #[instrument(skip(self))]
    pub(crate) async fn save(&self, shard: Shard, report: SignedReport) -> Result<String, ErrReport> {
        debug!("got report");
        let now = ReportTimestamp::now()?;
        let mut map = self.map.lock().unwrap();
        match map.entry(shard).or_default().entry(now).or_default() {
            StorageEntry::Open(ref mut reports) => {
                report
                    .clone()
                    .verify()
                    .set_status(StatusCode::BAD_REQUEST)?;
                reports.push(report);
                Ok(format!("report saved"))
            }
            StorageEntry::Sealed(_) => {
                Err(eyre!("Current entry is already sealed. Is time broken?"))
                    .set_status(StatusCode::CONFLICT)?
            }
        }
    }

    #[instrument(skip(self))]
    pub(crate) async fn get(&self, shard: Shard, timeframe: ReportTimestamp) -> Result<Vec<u8>, ErrReport> {
        debug!(?timeframe, "got request for entries");
        // Reject requests for the current timeframe.
        let current = ReportTimestamp::now()?;
        if timeframe == current {
            return Err(eyre!("Cannot request entries for current timeframe"))
                .set_status(StatusCode::FORBIDDEN)?;
        }

        let mut map = self.map.lock().unwrap();
        let entry = map
	    .get_mut(&shard)
	    .ok_or(eyre!("No entries for this shard"))
            .set_status(StatusCode::NOT_FOUND)?
            .get_mut(&timeframe)
            .ok_or(eyre!("No entries for this timeframe"))
            .set_status(StatusCode::NOT_FOUND)?;

        // We already checked that it's not the current timeframe, so if we see
        // StorageEntry::Open, seal it:
        entry.seal();

        if let StorageEntry::Sealed(ref bytes) = entry {
            Ok(bytes.clone())
        } else {
            Err(eyre!("Could not seal report batch"))
                .set_status(StatusCode::INTERNAL_SERVER_ERROR)?
        }
    }
}

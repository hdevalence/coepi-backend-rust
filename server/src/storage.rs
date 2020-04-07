use super::{error::ErrReport, ReportTimestamp, SignedReport};
use crate::error::context::Status;
use eyre::eyre;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Mutex;
use tracing::instrument;
use warp::http::StatusCode;

#[derive(Default)]
pub(crate) struct StorageEntry {
    reports: Vec<SignedReport>,
    bytes: Option<Vec<u8>>,
}

#[derive(Default)]
pub struct Storage {
    map: Mutex<HashMap<ReportTimestamp, StorageEntry>>,
}

impl Storage {
    #[instrument(skip(self))]
    pub(crate) async fn save(&self, report: SignedReport) -> Result<String, ErrReport> {
        let now = ReportTimestamp::now()?;
        let mut map = self.map.lock().unwrap();
        let entry = map.entry(now).or_default();
        if entry.bytes.is_some() {
            Err(eyre!(
                "Attempted to save for entry that has been read from. Is time broken?"
            ))
            .set_status(StatusCode::CONFLICT)?;
        }
        entry.reports.push(report);
        Ok(format!("report saved"))
    }

    #[instrument(skip(self))]
    pub(crate) async fn get(&self, timeframe: ReportTimestamp) -> Result<Vec<u8>, ErrReport> {
        let mut map = self.map.lock().unwrap();
        let entry = map
            .get_mut(&timeframe)
            .ok_or(eyre!("No entries for this timestamp"))
            .set_status(StatusCode::NOT_FOUND)?;

        if entry.bytes.is_none() {
            entry.reports.shuffle(&mut OsRng);

            let mut bytes: Vec<u8> = vec![];

            for e in &entry.reports {
                e.write(&mut bytes)?;
            }

            entry.bytes = Some(bytes);
        }

        Ok(entry.bytes.clone().unwrap())
    }
}

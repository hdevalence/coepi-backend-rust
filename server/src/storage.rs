use super::{ReportTimestamp, SignedReport};
use eyre::{eyre, ErrReport};
use rand::seq::SliceRandom;
use rand::rngs::OsRng;
use std::collections::HashMap;

#[derive(Default)]
struct StorageEntry {
    reports: Vec<SignedReport>,
    bytes: Option<Vec<u8>>,
}

#[derive(Default)]
pub struct Storage {
    map: HashMap<ReportTimestamp, StorageEntry>
}

impl Storage {
    pub(crate) fn save(&mut self, report: SignedReport) -> Result<(), ErrReport> {
        let now = ReportTimestamp::now()?;
        let entry = self.map.entry(now).or_default();
        if entry.bytes.is_some() {
            return Err(eyre!("Attempted to save for entry that has been read from. Is time broken?"))
        }
        entry.reports.push(report);
        Ok(())
    }

    pub(crate) fn get(&mut self, timeframe: ReportTimestamp) -> Result<Vec<u8>, ErrReport> {
        let entry = self.map.get_mut(&timeframe).ok_or(eyre!("No entries for this timestamp"))?;
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

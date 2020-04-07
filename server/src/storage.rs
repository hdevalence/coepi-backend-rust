use super::{CoepiReport, ReportTimestamp, SignedReport};
use eyre::ErrReport;

pub(crate) fn save(report: SignedReport) -> Result<(), ErrReport> {
    unimplemented!();
}

pub(crate) fn get(timeframe: ReportTimestamp) -> Result<Vec<u8>, ErrReport> {
    unimplemented!();
}

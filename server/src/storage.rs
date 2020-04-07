use super::{ReportTimeframe, SignedReport};
use eyre::ErrReport;

pub(crate) fn save(report: SignedReport) -> Result<(), ErrReport> {
    unimplemented!();
}

pub(crate) fn get(timeframe: ReportTimeframe) -> Result<Vec<u8>, ErrReport> {
    unimplemented!();
}

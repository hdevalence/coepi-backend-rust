use super::{CoepiReport, ReportTimeframe};
use eyre::ErrReport;

pub(crate) fn save(report: CoepiReport) -> Result<(), ErrReport> {
    unimplemented!();
}

pub(crate) fn get(timeframe: ReportTimeframe) -> Result<Vec<CoepiReport>, ErrReport> {
    unimplemented!();
}


use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct ReportTimestamp(pub u64);


impl ReportTimestamp {
    // half a day
    pub const SECONDS_PER_TIMESTAMP: u64 = 60 * 60 * 12;

    pub fn start_time(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(self.0 * Self::SECONDS_PER_TIMESTAMP)
    }

    pub fn end_time(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs((self.0 + 1) * Self::SECONDS_PER_TIMESTAMP) - Duration::from_nanos(1)
    }

    pub fn now() -> Result<Self, SystemTimeError> {
        Self::from_time(SystemTime::now())
    }

    pub fn from_time(t: SystemTime) -> Result<Self, SystemTimeError> {
        Ok(Self(t.duration_since(SystemTime::UNIX_EPOCH)?.as_secs() / Self::SECONDS_PER_TIMESTAMP))
    }
}

#[test]
fn test_timestamp() {
    let ts = ReportTimestamp::now().unwrap();
    assert!(ts.start_time() < ts.end_time());
    assert!(ReportTimestamp(ts.0 + 1).start_time() > ts.end_time());
    assert!(ReportTimestamp(ts.0 + 1).start_time().duration_since(ts.end_time()).unwrap().as_nanos() == 1);
    assert!(ReportTimestamp::from_time(ts.start_time()).unwrap() == ts);
    assert!(ReportTimestamp::from_time(ts.end_time()).unwrap() == ts);
}

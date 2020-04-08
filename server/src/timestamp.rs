use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct ReportTimestamp(pub u64);

impl std::str::FromStr for ReportTimestamp {
    type Err = std::num::ParseIntError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        Ok(Self(input.parse()?))
    }
}

use super::OPTIONS;

impl ReportTimestamp {
    pub fn start_time(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(self.0 * OPTIONS.seconds_per_batch)
    }

    pub fn end_time(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs((self.0 + 1) * OPTIONS.seconds_per_batch)
            - Duration::from_nanos(1)
    }

    pub fn now() -> Result<Self, SystemTimeError> {
        Self::from_time(SystemTime::now())
    }

    pub fn from_time(t: SystemTime) -> Result<Self, SystemTimeError> {
        Ok(Self(
            t.duration_since(SystemTime::UNIX_EPOCH)?.as_secs() / OPTIONS.seconds_per_batch,
        ))
    }
}

#[test]
fn test_timestamp() {
    let ts = ReportTimestamp::now().unwrap();
    assert!(ts.start_time() < ts.end_time());
    assert!(ReportTimestamp(ts.0 + 1).start_time() > ts.end_time());
    assert!(
        ReportTimestamp(ts.0 + 1)
            .start_time()
            .duration_since(ts.end_time())
            .unwrap()
            .as_nanos()
            == 1
    );
    assert!(ReportTimestamp::from_time(ts.start_time()).unwrap() == ts);
    assert!(ReportTimestamp::from_time(ts.end_time()).unwrap() == ts);
}

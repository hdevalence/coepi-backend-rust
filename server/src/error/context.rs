use super::ErrReport;
use eyre::Chain;
use indenter::Indented;
use std::fmt::Write;
use tracing_error::SpanTrace;
use tracing_error::SpanTraceStatus;
use warp::http::StatusCode;

pub(crate) struct Context {
    pub(crate) status: StatusCode,
    span_trace: SpanTrace,
}

impl eyre::EyreContext for Context {
    fn default(_: &(dyn std::error::Error + 'static)) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            span_trace: SpanTrace::capture(),
        }
    }

    fn debug(
        &self,
        error: &(dyn std::error::Error + 'static),
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        if f.alternate() {
            return core::fmt::Debug::fmt(error, f);
        }

        write!(f, "{}", self.status)?;

        let errors = Chain::new(error).rev().enumerate();

        for (n, error) in errors {
            writeln!(f)?;
            write!(Indented::numbered(f, n), "{}", error)?;
        }

        let span_trace = &self.span_trace;

        match span_trace.status() {
            SpanTraceStatus::CAPTURED => write!(f, "\n\nSpan Trace:\n{}", span_trace)?,
            SpanTraceStatus::UNSUPPORTED => write!(f, "\n\nWarning: SpanTrace capture is Unsupported.\nEnsure that you've setup an error layer and the versions match")?,
            _ => (),
        }

        Ok(())
    }
}

pub(crate) trait Status {
    type Result;
    fn set_status(self, status: StatusCode) -> Self::Result;
}

impl<T, E> Status for Result<T, E>
where
    E: Into<ErrReport>,
{
    type Result = Result<T, ErrReport>;

    fn set_status(self, status: StatusCode) -> Self::Result {
        self.map_err(|e| {
            let mut reporter = e.into();
            reporter.0.context_mut().status = status;
            reporter
        })
    }
}

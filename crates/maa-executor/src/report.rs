use crate::{Error, ErrorKind, Result};

/// Final status of one session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionStatus {
    Succeeded,
    Failed,
    Interrupted,
}

/// Structured failure retained in a session report after execution continues.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionFailure {
    pub kind: ErrorKind,
    pub message: String,
}

impl SessionFailure {
    pub(crate) fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub(crate) fn from_error(error: Error) -> Self {
        let kind = error.kind();
        let message = error
            .description()
            .map(str::to_owned)
            .unwrap_or_else(|| error.to_string());
        Self { kind, message }
    }
}

/// Final report for one session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionReport {
    pub name: Option<String>,
    pub status: SessionStatus,
    pub failure: Option<SessionFailure>,
}

/// Final status of the complete execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionStatus {
    Succeeded,
    Failed,
    Interrupted,
}

/// Final report for all sessions in one execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionReport {
    pub status: ExecutionStatus,
    pub sessions: Vec<SessionReport>,
}

impl ExecutionReport {
    /// Converts the report status into a structured executor result.
    pub fn result(&self) -> Result<()> {
        match self.status {
            ExecutionStatus::Succeeded => Ok(()),
            ExecutionStatus::Interrupted => {
                Err(Error::new(ErrorKind::Interrupted).with_desc("execution interrupted"))
            }
            ExecutionStatus::Failed => {
                if let Some(failed) = self
                    .sessions
                    .iter()
                    .find(|report| matches!(report.status, SessionStatus::Failed))
                {
                    let name = failed
                        .name
                        .clone()
                        .unwrap_or_else(|| String::from("unnamed"));
                    if let Some(failure) = &failed.failure {
                        return Err(Error::new(failure.kind)
                            .with_desc(format!("session `{name}` failed: {}", failure.message)));
                    }
                    return Err(Error::new(ErrorKind::Execution)
                        .with_desc(format!("session `{name}` failed")));
                }
                Err(Error::new(ErrorKind::Execution).with_desc("execution failed"))
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn failed_report_preserves_session_failure_kind() {
        let report = ExecutionReport {
            status: ExecutionStatus::Failed,
            sessions: vec![SessionReport {
                name: Some(String::from("main")),
                status: SessionStatus::Failed,
                failure: Some(SessionFailure::new(ErrorKind::Connection, "offline")),
            }],
        };

        let error = report.result().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::Connection);
        assert_eq!(
            error.to_string(),
            "connection failed: session `main` failed: offline"
        );
    }
}

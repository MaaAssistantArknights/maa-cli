use maa_ffi_types::AsstMsgId;

/// The kind of message received in the assistant callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    /* Global Info */
    InternalError,
    InitFailed,
    ConnectionInfo,
    AllTasksCompleted,
    AsyncCallInfo,
    Destroyed,

    /* TaskChain Info */
    TaskChainError,
    TaskChainStart,
    TaskChainCompleted,
    TaskChainExtraInfo,
    TaskChainStopped,

    /* SubTask Info */
    SubTaskError,
    SubTaskStart,
    SubTaskCompleted,
    SubTaskExtraInfo,
    SubTaskStopped,

    /* External Callback */
    ReportRequest,

    /// An unknown message kind; the original ID is preserved for debugging.
    Unknown(AsstMsgId),
}

impl From<AsstMsgId> for MessageKind {
    fn from(id: AsstMsgId) -> Self {
        match id {
            0 => MessageKind::InternalError,
            1 => MessageKind::InitFailed,
            2 => MessageKind::ConnectionInfo,
            3 => MessageKind::AllTasksCompleted,
            4 => MessageKind::AsyncCallInfo,
            5 => MessageKind::Destroyed,

            10000 => MessageKind::TaskChainError,
            10001 => MessageKind::TaskChainStart,
            10002 => MessageKind::TaskChainCompleted,
            10003 => MessageKind::TaskChainExtraInfo,
            10004 => MessageKind::TaskChainStopped,

            20000 => MessageKind::SubTaskError,
            20001 => MessageKind::SubTaskStart,
            20002 => MessageKind::SubTaskCompleted,
            20003 => MessageKind::SubTaskExtraInfo,
            20004 => MessageKind::SubTaskStopped,

            30000 => MessageKind::ReportRequest,

            id => MessageKind::Unknown(id),
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn from_known_ids() {
        assert_eq!(MessageKind::from(0), MessageKind::InternalError);
        assert_eq!(MessageKind::from(1), MessageKind::InitFailed);
        assert_eq!(MessageKind::from(2), MessageKind::ConnectionInfo);
        assert_eq!(MessageKind::from(3), MessageKind::AllTasksCompleted);
        assert_eq!(MessageKind::from(4), MessageKind::AsyncCallInfo);
        assert_eq!(MessageKind::from(5), MessageKind::Destroyed);
        assert_eq!(MessageKind::from(10000), MessageKind::TaskChainError);
        assert_eq!(MessageKind::from(10001), MessageKind::TaskChainStart);
        assert_eq!(MessageKind::from(10002), MessageKind::TaskChainCompleted);
        assert_eq!(MessageKind::from(10003), MessageKind::TaskChainExtraInfo);
        assert_eq!(MessageKind::from(10004), MessageKind::TaskChainStopped);
        assert_eq!(MessageKind::from(20000), MessageKind::SubTaskError);
        assert_eq!(MessageKind::from(20001), MessageKind::SubTaskStart);
        assert_eq!(MessageKind::from(20002), MessageKind::SubTaskCompleted);
        assert_eq!(MessageKind::from(20003), MessageKind::SubTaskExtraInfo);
        assert_eq!(MessageKind::from(20004), MessageKind::SubTaskStopped);
        assert_eq!(MessageKind::from(30000), MessageKind::ReportRequest);
    }

    #[test]
    fn unknown_id_preserved() {
        assert_eq!(MessageKind::from(9999), MessageKind::Unknown(9999));
    }
}

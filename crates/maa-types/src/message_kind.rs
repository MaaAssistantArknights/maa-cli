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

use std::sync::mpsc;

use maa_types::MessageKind;
use serde_json::Value;

use crate::{ErrorKind, ExecutionStatus, SessionReport};

/// A MaaCore callback associated with an executor session when possible.
#[derive(Clone, Debug, PartialEq)]
pub struct CoreEvent {
    /// The session that owns the callback task, or `None` for instance-level callbacks.
    pub session: Option<usize>,
    /// MaaCore's callback message kind.
    pub kind: MessageKind,
    /// MaaCore's callback payload.
    pub payload: Value,
}

/// A lifecycle or MaaCore callback event emitted by an execution.
///
/// Exactly one terminal event is emitted: [`ExecutionEvent::Finished`] when an
/// [`crate::ExecutionReport`] is available, or [`ExecutionEvent::Failed`] when execution ends
/// before a report can be produced.
#[derive(Clone, Debug, PartialEq)]
pub enum ExecutionEvent {
    EnvironmentPreparing,
    EnvironmentPrepared { address: Option<String> },
    Connecting { address: String },
    Connected,
    SessionStarted { index: usize, name: Option<String> },
    Core(CoreEvent),
    SessionFinished { index: usize, report: SessionReport },
    Interrupted,
    Failed { kind: ErrorKind, message: String },
    Finished { status: ExecutionStatus },
}

#[derive(Clone)]
pub(crate) struct EventSender {
    sender: mpsc::Sender<ExecutionEvent>,
}

impl EventSender {
    pub(crate) fn send(&self, event: ExecutionEvent) {
        // Event consumption is optional; dropping it must not stop execution.
        let _ = self.sender.send(event);
    }
}

pub(crate) fn channel() -> (EventSender, ExecutionSubscription) {
    let (sender, receiver) = mpsc::channel();
    (EventSender { sender }, ExecutionSubscription { receiver })
}

/// The single ordered event stream for an execution.
pub struct ExecutionSubscription {
    receiver: mpsc::Receiver<ExecutionEvent>,
}

impl ExecutionSubscription {
    /// Waits for the next event.
    pub fn recv(&self) -> Result<ExecutionEvent, mpsc::RecvError> {
        self.receiver.recv()
    }

    /// Attempts to receive the next event without blocking.
    pub fn try_recv(&self) -> Result<ExecutionEvent, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Iterates until execution drops all event senders.
    pub fn iter(&self) -> mpsc::Iter<'_, ExecutionEvent> {
        self.receiver.iter()
    }
}

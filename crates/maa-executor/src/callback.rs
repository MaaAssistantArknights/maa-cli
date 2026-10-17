use std::{
    collections::HashMap,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use log::warn;
use maa_core::Callback;
use maa_types::MessageKind;
use serde_json::Value;

use crate::events::{CoreEvent, EventSender, ExecutionEvent};

pub(crate) struct SessionRuntimeState {
    index: usize,
    core_errored: AtomicBool,
    offline_stop_requested: AtomicBool,
    terminal: Mutex<bool>,
    terminal_changed: Condvar,
}

impl SessionRuntimeState {
    fn new(index: usize) -> Self {
        Self {
            index,
            core_errored: AtomicBool::new(false),
            offline_stop_requested: AtomicBool::new(false),
            terminal: Mutex::new(false),
            terminal_changed: Condvar::new(),
        }
    }

    pub(crate) fn core_errored(&self) -> bool {
        self.core_errored.load(Ordering::Relaxed)
    }

    pub(crate) fn offline_stop_requested(&self) -> bool {
        self.offline_stop_requested.load(Ordering::Relaxed)
    }

    pub(crate) fn wait_for_terminal_callback(&self) {
        let mut terminal = self.terminal.lock().unwrap();
        while !*terminal {
            terminal = self.terminal_changed.wait(terminal).unwrap();
        }
    }

    fn mark_terminal(&self) {
        *self.terminal.lock().unwrap() = true;
        self.terminal_changed.notify_all();
    }
}

pub(crate) struct CallbackRuntime {
    auto_reconnect: bool,
    task_sessions: Mutex<HashMap<i64, Arc<SessionRuntimeState>>>,
    events: EventSender,
}

impl CallbackRuntime {
    pub(crate) fn new(auto_reconnect: bool, events: EventSender) -> Self {
        Self {
            auto_reconnect,
            task_sessions: Mutex::new(HashMap::new()),
            events,
        }
    }

    pub(crate) fn create_session(&self, index: usize) -> Arc<SessionRuntimeState> {
        Arc::new(SessionRuntimeState::new(index))
    }

    pub(crate) fn register_task(&self, task_id: i32, session: &Arc<SessionRuntimeState>) {
        self.task_sessions
            .lock()
            .unwrap()
            .insert(i64::from(task_id), Arc::clone(session));
    }

    fn session_for(&self, payload: &Value) -> Option<Arc<SessionRuntimeState>> {
        let task_id = payload.get("taskid").and_then(Value::as_i64).or_else(|| {
            payload
                .get("finished_tasks")
                .and_then(Value::as_array)
                .and_then(|tasks| tasks.last())
                .and_then(Value::as_i64)
        })?;

        self.task_sessions.lock().unwrap().get(&task_id).cloned()
    }

    fn process(&self, kind: MessageKind, payload: Value) {
        let session = self.session_for(&payload);
        let terminal = matches!(
            kind,
            MessageKind::AllTasksCompleted | MessageKind::TaskChainStopped
        );

        if let Some(session) = &session {
            match kind {
                MessageKind::InternalError
                | MessageKind::InitFailed
                | MessageKind::TaskChainError
                | MessageKind::TaskChainStopped => {
                    session.core_errored.store(true, Ordering::Relaxed);
                }
                MessageKind::SubTaskStart
                    if !self.auto_reconnect && is_offline_confirm(&payload) =>
                {
                    session
                        .offline_stop_requested
                        .store(true, Ordering::Relaxed);
                }
                _ => {}
            }
        }

        self.events.send(ExecutionEvent::Core(CoreEvent {
            session: session.as_ref().map(|session| session.index),
            kind,
            payload,
        }));

        // Wake the runner after forwarding the terminal callback so SessionFinished cannot
        // overtake the final MaaCore event.
        if terminal && let Some(session) = session {
            session.mark_terminal();
        }
    }
}

fn is_offline_confirm(payload: &Value) -> bool {
    payload.get("subtask").and_then(Value::as_str) == Some("ProcessTask")
        && payload
            .get("details")
            .and_then(|details| details.get("task"))
            .and_then(Value::as_str)
            == Some("OfflineConfirm")
}

pub(crate) struct MaaCallback {
    runtime: Arc<CallbackRuntime>,
}

impl MaaCallback {
    pub(crate) fn new(runtime: Arc<CallbackRuntime>) -> Self {
        Self { runtime }
    }
}

impl Callback for MaaCallback {
    fn on_message(&self, kind: MessageKind, msg: Option<&str>) {
        let Some(message) = msg else {
            self.runtime.process(kind, Value::Null);
            return;
        };
        let Ok(payload) = serde_json::from_str::<Value>(message) else {
            warn!("Failed to parse MaaCore callback {kind:?}: {message}");
            return;
        };
        self.runtime.process(kind, payload);
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use maa_types::MessageKind;

    use super::*;
    use crate::{ExecutionEvent, events::channel};

    fn callback(
        auto_reconnect: bool,
    ) -> (
        Arc<CallbackRuntime>,
        MaaCallback,
        crate::ExecutionSubscription,
    ) {
        let (events, subscription) = channel();
        let runtime = Arc::new(CallbackRuntime::new(auto_reconnect, events));
        let callback = MaaCallback::new(Arc::clone(&runtime));
        (runtime, callback, subscription)
    }

    fn offline_confirm_msg(task_id: i32) -> String {
        format!(
            r#"{{"taskid":{task_id},"subtask":"ProcessTask","details":{{"task":"OfflineConfirm"}}}}"#
        )
    }

    #[test]
    fn offline_confirm_stops_when_auto_reconnect_disabled() {
        let (runtime, callback, _events) = callback(false);
        let session = runtime.create_session(0);
        runtime.register_task(7, &session);

        callback.on_message(MessageKind::SubTaskStart, Some(&offline_confirm_msg(7)));

        assert!(session.offline_stop_requested());
    }

    #[test]
    fn offline_confirm_does_not_stop_when_auto_reconnect_enabled() {
        let (runtime, callback, _events) = callback(true);
        let session = runtime.create_session(0);
        runtime.register_task(7, &session);

        callback.on_message(MessageKind::SubTaskStart, Some(&offline_confirm_msg(7)));

        assert!(!session.offline_stop_requested());
    }

    #[test]
    fn forwards_core_event_to_owning_session() {
        let (runtime, callback, events) = callback(true);
        let session = runtime.create_session(2);
        runtime.register_task(7, &session);

        callback.on_message(
            MessageKind::SubTaskStart,
            Some(r#"{"taskid":7,"subtask":"ProcessTask","details":{"task":"StartButton2"}}"#),
        );

        assert!(matches!(
            events.recv().unwrap(),
            ExecutionEvent::Core(CoreEvent {
                session: Some(2),
                kind: MessageKind::SubTaskStart,
                ..
            })
        ));
    }

    #[test]
    fn instance_event_has_no_session() {
        let (_runtime, callback, events) = callback(true);

        callback.on_message(MessageKind::ConnectionInfo, Some(r#"{"what":"Connected"}"#));

        assert!(matches!(
            events.recv().unwrap(),
            ExecutionEvent::Core(CoreEvent {
                session: None,
                kind: MessageKind::ConnectionInfo,
                ..
            })
        ));
    }

    #[test]
    fn delayed_callback_does_not_mutate_next_session() {
        let (runtime, callback, events) = callback(true);
        let first = runtime.create_session(0);
        let second = runtime.create_session(1);
        runtime.register_task(7, &first);
        runtime.register_task(8, &second);

        callback.on_message(
            MessageKind::TaskChainError,
            Some(r#"{"taskid":7,"taskchain":"Fight"}"#),
        );

        assert!(first.core_errored());
        assert!(!second.core_errored());
        assert!(matches!(
            events.recv().unwrap(),
            ExecutionEvent::Core(CoreEvent {
                session: Some(0),
                kind: MessageKind::TaskChainError,
                ..
            })
        ));
    }

    #[test]
    fn terminal_state_is_set_after_event_is_forwarded() {
        let (runtime, callback, events) = callback(true);
        let session = runtime.create_session(0);
        runtime.register_task(7, &session);

        callback.on_message(
            MessageKind::AllTasksCompleted,
            Some(r#"{"finished_tasks":[7]}"#),
        );
        session.wait_for_terminal_callback();

        assert!(matches!(
            events.try_recv().unwrap(),
            ExecutionEvent::Core(CoreEvent {
                session: Some(0),
                kind: MessageKind::AllTasksCompleted,
                ..
            })
        ));
    }

    #[test]
    fn unexpected_task_stop_marks_session_failed() {
        let (runtime, callback, _events) = callback(true);
        let session = runtime.create_session(0);
        runtime.register_task(7, &session);

        callback.on_message(
            MessageKind::TaskChainStopped,
            Some(r#"{"taskid":7,"taskchain":"Fight"}"#),
        );

        assert!(session.core_errored());
        session.wait_for_terminal_callback();
    }
}

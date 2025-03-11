use std::collections::BTreeMap;

use parking_lot::RwLock;
use tokio::sync::oneshot::Sender;

use crate::types::{SessionID, TaskId, TaskStateType};

static SESSION_POOL: RwLock<BTreeMap<SessionID, _Session>> = RwLock::new(BTreeMap::new());

type LogContent = (TaskStateType, String);
type CallBackContent = String;

// re-export
pub use state::State;

pub trait SessionExt: Sized {
    fn as_id(self) -> SessionID;

    /// Create a [Session] with given `callback` and insert with `session_id`
    fn add(self, callback: Sender<log::CallBack>) {
        let session_id = self.as_id();
        let session = _Session::new(callback);
        SESSION_POOL.write().insert(session_id, session);
    }

    /// Remove [Session] with given `session_id`
    ///
    /// Return [false] if no such one
    fn remove(self) -> bool {
        let session_id = self.as_id();
        SESSION_POOL.write().remove(&session_id).is_some()
    }

    /// Take the rx side to create a `Stream`` to client
    ///
    /// Return [None] if already taken
    fn take_subscriber(self) -> Option<tokio::sync::mpsc::UnboundedReceiver<LogContent>> {
        let session_id = self.as_id();
        SESSION_POOL
            .write()
            .get_mut(&session_id)
            .and_then(|logger| logger.channel.take_rx())
    }

    /// safety: this should be called only during Task::new_connection
    fn test_connection_result(self, err: Option<CallBackContent>) {
        let session_id = self.as_id();
        if let Some(err) = err {
            SESSION_POOL
                .write()
                .get_mut(&session_id)
                .unwrap()
                .channel
                .connect_failed(err);
        } else {
            SESSION_POOL
                .write()
                .get_mut(&session_id)
                .unwrap()
                .channel
                .connect_success();
        }
    }

    fn info_to_channel(self, msg: LogContent) {
        let session_id = self.as_id();
        SESSION_POOL
            .read()
            .get(&session_id)
            .unwrap()
            .channel
            .log_to_channel(msg);
    }

    fn tasks(self) -> Tasks {
        Tasks(self.as_id())
    }

    fn log(self) -> Log {
        Log(self.as_id())
    }
}

impl SessionExt for SessionID {
    fn as_id(self) -> SessionID {
        todo!()
    }
}

/// Wrapper around [SessionID],
/// providing function about tasks
pub struct Tasks(SessionID);
impl Tasks {
    #[allow(clippy::new_ret_no_self, clippy::wrong_self_convention)]
    pub fn new(self, task_id: TaskId) {
        if let Some(session) = SESSION_POOL.write().get_mut(&self.0) {
            session.new_task(task_id);
        }
    }

    pub fn state(self, task_id: TaskId, new: state::State) {
        if let Some(state) = SESSION_POOL
            .write()
            .get_mut(&self.0)
            .and_then(|session| session.tasks.get_mut(&task_id))
        {
            state.reason(new);
        }
    }

    pub fn update(self, task_id: TaskId, message: LogContent) {
        if let Some(session) = SESSION_POOL.write().get_mut(&self.0) {
            session
                .tasks
                .get_mut(&task_id)
                .unwrap()
                .update(message.clone());
            session.channel.log_to_channel(message);
        }
    }
}

/// Wrapper around [SessionID],
/// providing function about log
pub struct Log(SessionID);
impl Log {
    pub fn get_skip(self, len: i32) -> Vec<LogContent> {
        if let Some(session) = SESSION_POOL.read().get(&self.0) {
            session.get_skip_len(len as usize)
        } else {
            vec![]
        }
    }

    pub fn log(self, message: LogContent) {
        if let Some(session) = SESSION_POOL.write().get_mut(&self.0) {
            session.log(message)
        } else {
            tracing::warn!(from = ?self.0, "Unknown Log: {:?}", message)
        }
    }
}

struct _Session {
    tasks: BTreeMap<TaskId, state::TaskState>,
    channel: log::Channel,
    logs: Vec<LogContent>,
}

impl _Session {
    fn new(callback: tokio::sync::oneshot::Sender<log::CallBack>) -> Self {
        Self {
            tasks: Default::default(),
            channel: log::Channel::new(callback),
            logs: Default::default(),
        }
    }

    fn new_task(&mut self, task_id: TaskId) {
        self.tasks.insert(task_id, state::TaskState::default());
    }

    fn log(&mut self, log: LogContent) {
        self.logs.push(log);
    }

    fn get_skip_len(&self, len: usize) -> Vec<LogContent> {
        self.logs.iter().skip(len).cloned().collect()
    }
}

mod log {
    use tokio::sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot,
    };

    use super::{CallBackContent, LogContent};

    pub type Channel = Logger<LogContent, CallBack>;
    pub(super) type CallBack = Result<(), CallBackContent>;

    pub struct Logger<T, R> {
        tx: UnboundedSender<T>,
        rx: Option<UnboundedReceiver<T>>,
        /// used for check adb connection
        oneshot: Option<oneshot::Sender<R>>,
    }
    impl Logger<LogContent, CallBack> {
        pub(super) fn new(oneshot: oneshot::Sender<CallBack>) -> Self {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            Self {
                tx,
                rx: Some(rx),
                oneshot: Some(oneshot),
            }
        }

        pub fn take_rx(&mut self) -> Option<UnboundedReceiver<LogContent>> {
            self.rx.take()
        }

        pub(super) fn log_to_channel(&self, message: LogContent) {
            let _ = self.tx.send(message);
        }

        pub fn connect_failed(&mut self, err: CallBackContent) {
            if let Some(shot) = self.oneshot.take() {
                let _ = shot.send(Err(err));
            }
        }

        pub fn connect_success(&mut self) {
            if let Some(shot) = self.oneshot.take() {
                let _ = shot.send(Ok(()));
            }
        }
    }
}

mod state {
    use super::LogContent;

    #[derive(Debug, Clone, Default)]
    pub struct TaskState {
        state: State,
        content: Vec<LogContent>,
    }

    #[cfg_attr(debug_assertions, derive(PartialEq))]
    #[derive(Debug, Clone, Copy, Default)]
    pub enum State {
        #[default]
        Waiting,
        Running,
        Completed,
        Canceled,
        Error,
    }

    impl TaskState {
        pub fn reason(&mut self, reason: State) {
            debug_assert_eq!(self.state, match reason {
                State::Waiting => unreachable!(),
                State::Running => State::Waiting,
                State::Completed | State::Canceled | State::Error => State::Running,
            });
            self.state = reason;
        }

        pub fn update(&mut self, new: LogContent) {
            self.content.push(new);
        }
    }
}

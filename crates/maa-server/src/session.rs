use std::collections::BTreeMap;

use parking_lot::Mutex;
use tokio::sync::oneshot::Sender;

use crate::{
    error::NO_SUCH_SESSION,
    types::{SessionID, TaskId, TaskStateType},
};

type MMap<S, ID = SessionID> = Mutex<BTreeMap<ID, S>>;
type LogContent = (TaskStateType, String);
type CallBackContent = String;
type CallBack = Result<(), CallBackContent>;
type Result<T, E = Box<dyn core::error::Error + Send + Sync>> = core::result::Result<T, E>;

static SESSION_POOL: MMap<Session> = Mutex::new(BTreeMap::new());
static CONNECT_POOL: MMap<Sender<CallBack>> = Mutex::new(BTreeMap::new());

// re-export
pub use state::State;

pub trait SessionExt: Sized {
    fn as_id(&self) -> SessionID;

    /// Insert the [Session] with current `session_id`
    fn add(self, asst: Assistant) {
        let session_id = self.as_id();
        let session = Session::new(asst);
        SESSION_POOL.lock().insert(session_id, session);
    }

    /// Remove [Session] with current `session_id`
    ///
    /// Return [false] if already dropped
    fn remove(self) -> bool {
        let session_id = self.as_id();
        tracing::info!("Terminate Session");
        // here we must let lock have a binding, or this will block the thread
        // I don't know why, but this just work
        // hope it won't be worse later
        let mut lock = SESSION_POOL.lock();
        lock.remove(&session_id).is_some()
        // SESSION_POOL.lock().remove(&session_id).is_some()
    }

    fn tasks(self) -> Tasks {
        Tasks(self.as_id())
    }

    fn log(self) -> Log {
        Log(self.as_id())
    }

    fn adb(self) -> InitAdb {
        InitAdb(self.as_id())
    }
}

impl SessionExt for SessionID {
    fn as_id(&self) -> SessionID {
        *self
    }
}

pub struct InitAdb(SessionID);
impl InitAdb {
    pub fn register(self, callback: Sender<CallBack>) {
        let session_id = self.0;
        CONNECT_POOL.lock().insert(session_id, callback);
    }

    /// safety: this should be called only during Task::new_connection
    fn result(self, err: Option<CallBackContent>) {
        let session_id = self.0;
        if let Some(shot) = CONNECT_POOL.lock().remove(&session_id) {
            if shot
                .send(match err {
                    Some(err) => Err(err),
                    None => Ok(()),
                })
                .is_err()
            {
                tracing::error!("CallBack Rx dropped before tx send.");
            }
        } else {
            tracing::error!("Call result for more than once");
        }
    }

    pub fn success(self) {
        self.result(None);
    }

    pub fn fail(self, err: CallBackContent) {
        self.result(Some(err));
    }
}

/// Wrapper around [SessionID],
/// providing function about tasks
pub struct Tasks(SessionID);
impl Tasks {
    pub fn append(self, task_type: maa_sys::TaskType, params: &str) -> Result<TaskId> {
        let mut binding = SESSION_POOL.lock();
        let session = binding.get_mut(&self.0).ok_or(NO_SUCH_SESSION)?;
        let task_id = session
            .asst
            .inner_unchecked()
            .append_task(task_type, params)?;
        session.tasks.insert(task_id, state::TaskState::default());
        Ok(task_id)
    }

    pub fn start(self) -> Result<()> {
        let mut binding = SESSION_POOL.lock();
        let session = binding.get_mut(&self.0).ok_or(NO_SUCH_SESSION)?;
        session.asst.inner_unchecked().start()?;
        Ok(())
    }

    pub fn stop(self) -> Result<()> {
        let mut binding = SESSION_POOL.lock();
        let session = binding.get_mut(&self.0).ok_or(NO_SUCH_SESSION)?;
        session.asst.inner_unchecked().stop()?;
        Ok(())
    }

    pub fn patch_params(self, task_id: TaskId, params: &str) -> Result<()> {
        let mut binding = SESSION_POOL.lock();
        let session = binding.get_mut(&self.0).ok_or(NO_SUCH_SESSION)?;
        session
            .asst
            .inner_unchecked()
            .set_task_params(task_id, params)?;
        Ok(())
    }

    pub fn callback_state(self, task_id: TaskId, new: state::State) {
        if let Some(state) = SESSION_POOL
            .lock()
            .get_mut(&self.0)
            .and_then(|session| session.tasks.get_mut(&task_id))
        {
            state.reason(new);
        } else {
            tracing::warn!("State Update for Unknown Session")
        }
    }

    pub fn callback_log(self, task_id: TaskId, message: LogContent) {
        if let Some(session) = SESSION_POOL.lock().get_mut(&self.0) {
            if let Some(task_state) = session.tasks.get_mut(&task_id) {
                task_state.update(message.clone());
            } else {
                tracing::warn!(task_id = %task_id, "New Log for Unknown Task: {:?}", message)
            }
        } else {
            tracing::warn!("New Log for Unknown Session: {:?}", message)
        }
    }
}

/// Wrapper around [SessionID],
/// providing function about log
pub struct Log(SessionID);
impl Log {
    /// Take the rx side to create a `Stream`` to client
    ///
    /// Return [None] if allocky taken
    pub fn take_subscriber(self) -> Option<tokio::sync::mpsc::UnboundedReceiver<LogContent>> {
        let session_id = self.0;
        SESSION_POOL
            .lock()
            .get_mut(&session_id)
            .and_then(|logger| logger.channel.take_rx())
    }

    pub fn get_skip(self, len: usize) -> Vec<LogContent> {
        if let Some(session) = SESSION_POOL.lock().get(&self.0) {
            session.logs.iter().skip(len).cloned().collect()
        } else {
            vec![]
        }
    }

    pub fn send_to_channel(self, msg: LogContent) {
        let session_id = self.0;
        if let Some(session) = SESSION_POOL.lock().get(&session_id) {
            session.channel.log_to_channel(msg);
        } else {
            tracing::warn!("New Log for Unknown Session")
        }
    }

    pub fn log(self, message: LogContent) {
        if let Some(session) = SESSION_POOL.lock().get_mut(&self.0) {
            session.logs.push(message);
        } else {
            tracing::warn!("Unknown Log: {:?}", message)
        }
    }
}

struct Session {
    tasks: BTreeMap<TaskId, state::TaskState>,
    channel: log::Channel,
    logs: Vec<LogContent>,
    asst: Assistant,
}

impl Session {
    fn new(asst: Assistant) -> Self {
        Self {
            tasks: Default::default(),
            channel: log::Channel::new(),
            logs: Default::default(),
            asst,
        }
    }
}

mod log {
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

    use super::LogContent;

    pub type Channel = Logger<LogContent>;

    pub struct Logger<T> {
        tx: UnboundedSender<T>,
        rx: Option<UnboundedReceiver<T>>,
    }
    impl Logger<LogContent> {
        pub(super) fn new() -> Self {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            Self { tx, rx: Some(rx) }
        }

        pub fn take_rx(&mut self) -> Option<UnboundedReceiver<LogContent>> {
            self.rx.take()
        }

        pub(super) fn log_to_channel(&self, message: LogContent) {
            let _ = self.tx.send(message);
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

/// A wrapper for [`maa_sys::Assistant`]
pub struct Assistant {
    inner: maa_sys::Assistant,
}

unsafe impl Sync for Assistant {}

impl Assistant {
    pub fn new(session_id: SessionID) -> Self {
        let ptr = session_id.to_ptr();
        tracing::debug!(id = %session_id, "Forget here");
        Self {
            inner: maa_sys::Assistant::new(
                Some(crate::server_impl::default_callback),
                Some(ptr as *mut std::ffi::c_void),
            ),
        }
    }

    pub fn inner_unchecked(&self) -> &maa_sys::Assistant {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_adb() {
        let session = SessionID::new();

        let (tx, mut rx) = tokio::sync::oneshot::channel();
        session.adb().register(tx);
        assert_eq!(
            Err(tokio::sync::oneshot::error::TryRecvError::Empty),
            rx.try_recv()
        );

        session.adb().fail("err".to_owned());
        assert_eq!(Ok(Err("err".to_owned())), rx.try_recv());

        session.adb().success();
        assert_eq!(
            Err(tokio::sync::oneshot::error::TryRecvError::Closed),
            rx.try_recv()
        );
    }

    #[test]
    fn logger() {
        let mut logger = log::Logger::new();
        let mut rx = logger.take_rx().unwrap();
        assert!(matches!(logger.take_rx(), None));

        assert_eq!(
            Err(tokio::sync::mpsc::error::TryRecvError::Empty),
            rx.try_recv()
        );

        logger.log_to_channel((TaskStateType::Unknown, "content".to_owned()));
        assert_eq!(
            Ok((TaskStateType::Unknown, "content".to_owned())),
            rx.try_recv()
        );
    }
}

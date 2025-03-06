pub unsafe extern "C" fn default_callback(
    code: maa_types::primitive::AsstMsgId,
    json_str: *const std::ffi::c_char,
    session_id: *mut std::ffi::c_void,
) {
    let code: maa_server::callback::AsstMsg = code.into();
    let json_str = unsafe { std::ffi::CStr::from_ptr(json_str).to_str().unwrap() };
    let session_id: SessionIDRef = unsafe {
        std::ffi::CStr::from_ptr(session_id as *mut _ as *mut std::ffi::c_char)
            .to_str()
            .unwrap()
    };
    callback::main(code, json_str, session_id);
}

type SessionID = String;
/// an ugly way to make up
type SessionIDRef<'a> = &'a str;
use maa_types::primitive::AsstTaskId as TaskId;

type MaaUuid = String;

mod callback {
    use crate::{SessionIDRef, TaskId};
    use maa_server::callback::AsstMsg;
    use tracing::{debug, error, info, trace, warn};

    pub type Map = serde_json::Map<String, serde_json::Value>;

    #[tracing::instrument("C CallBack", skip_all)]
    pub fn main(code: maa_server::callback::AsstMsg, json_str: &str, session_id: SessionIDRef) {
        use crate::log::{Logger, TX_HANDLERS};
        tracing::trace!("Session ID: {}", session_id);

        if let Some(tx) = TX_HANDLERS.read().get(session_id) {
            if tx.log(json_str.to_string(), session_id.to_owned()) {
                TX_HANDLERS.write().remove(session_id);
            }
        } else {
            Logger::log_to_pool(json_str, session_id.to_owned());
        }

        let map: Map = serde_json::from_str(json_str).unwrap();

        // if ret is None, which means the message is not processed well
        // we should print the message to trace the error
        if process_message(code, map, session_id).is_none() {
            tracing::debug!(
                "FailedToProcessMessage, code: {:?}, message: {}",
                code,
                json_str
            )
        }
    }

    pub fn process_message(code: AsstMsg, message: Map, session_id: SessionIDRef) -> Option<()> {
        use AsstMsg::*;

        match code {
            InternalError => Some(()),
            InitFailed => {
                error!("InitializationError");
                Some(())
            }
            ConnectionInfo => process_connection_info(message, session_id),
            AllTasksCompleted => {
                info!("AllTasksCompleted");
                Some(())
            }
            AsyncCallInfo => Some(()),
            Destroyed => {
                debug!("Instance destroyed");
                Some(())
            }

            TaskChainError | TaskChainStart | TaskChainCompleted | TaskChainExtraInfo
            | TaskChainStopped => process_taskchain(code, message, session_id),

            SubTaskError | SubTaskStart | SubTaskCompleted | SubTaskExtraInfo | SubTaskStopped => {
                subtask::process_subtask(code, message, session_id)
            }

            Unknown => None,
        }
    }

    fn process_connection_info(message: Map, session_id: SessionIDRef) -> Option<()> {
        #[derive(serde::Deserialize)]
        struct ConnectionInfo {
            what: String,
            why: Option<String>,
            details: Map,
        }
        let ConnectionInfo { what, why, details } =
            serde_json::from_value(serde_json::Value::Object(message)).unwrap();

        match what.as_str() {
            "UuidGot" => {
                debug!("Got UUID: {}", details.get("uuid")?.as_str()?);
                // safety: this should be called only during Task::new_connection
                crate::log::TX_HANDLERS
                    .write()
                    .get_mut(session_id)
                    .unwrap()
                    .connect_success();
            }
            "ConnectFailed" => {
                let err = format!("Failed to connect to android device, {}, Please check your connect configuration: {}",
                    why.unwrap(),serde_json::to_string_pretty(&details).unwrap());
                error!(err);
                // safety: this should be called only during Task::new_connection
                crate::log::TX_HANDLERS
                    .write()
                    .remove(session_id)
                    .unwrap()
                    .connect_failed(err);
            }
            // Resolution
            "ResolutionGot" => trace!(
                "Got Resolution: {} X {}",
                details.get("width")?.as_i64()?,
                details.get("height")?.as_i64()?
            ),
            "UnsupportedResolution" => error!("Unsupported Resolution"),
            "ResolutionError" => error!("Resolution Acquisition Failure"),

            // Connection
            "Connected" => info!("Connected"),
            "Disconnect" => warn!("Disconnected"),
            "Reconnecting" => warn!(
                "{} {} {}",
                "Reconnect",
                details.get("times")?.as_i64()?,
                "times"
            ),
            "Reconnected" => info!("Reconnect Success"),

            // Screen Capture
            "ScreencapFailed" => error!("Screencap Failed"),
            "FastestWayToScreencap" => trace!(
                "{} {} {}",
                "Fastest Way To Screencap",
                details.get("method")?.as_str()?,
                details.get("cost")?.as_i64()?,
            ),
            "ScreencapCost" => trace!(
                "{} {} ({} ~ {})",
                "Screencap Cost",
                details.get("avg")?.as_i64()?,
                details.get("min")?.as_i64()?,
                details.get("max")?.as_i64()?,
            ),

            "TouchModeNotAvailable" => error!("Touch Mode Not Available"),
            _ => debug!(
                "{}: what:{} why:{} detials:{}",
                "Unknown Connection Info",
                what,
                why.as_deref().unwrap_or("No why"),
                serde_json::to_string_pretty(&details).unwrap()
            ),
        }

        Some(())
    }

    fn process_taskchain(code: AsstMsg, message: Map, session_id: SessionIDRef) -> Option<()> {
        #[derive(serde::Deserialize)]
        struct TaskChain {
            taskchain: maa_types::TaskType,
            taskid: TaskId,
        }
        let TaskChain { taskchain, taskid } =
            serde_json::from_value(serde_json::Value::Object(message)).unwrap();

        use crate::state::{Reason, StatePool};
        use AsstMsg::*;

        match code {
            TaskChainStart => {
                info!("{} {}", taskchain, "Start");
                StatePool::reason_task(session_id, taskid, Reason::Start);
            }
            TaskChainCompleted => {
                info!("{} {}", taskchain, "Completed");
                StatePool::reason_task(session_id, taskid, Reason::Complete);
            }
            TaskChainStopped => {
                warn!("{} {}", taskchain, "Stopped");
                StatePool::reason_task(session_id, taskid, Reason::Cancel);
            }
            TaskChainError => {
                error!("{} {}", taskchain, "Error");
                StatePool::reason_task(session_id, taskid, Reason::Error);
            }
            TaskChainExtraInfo => {}

            _ => {} // unreachable
        };

        Some(())
    }

    mod subtask {
        use super::*;

        pub fn process_subtask(
            _code: AsstMsg,
            message: Map,
            session_id: SessionIDRef,
        ) -> Option<()> {
            let msg = serde_json::to_string_pretty(&message).unwrap();
            let taskid = message.get("taskid")?.as_i64()? as TaskId;
            crate::state::StatePool::update_task(session_id, taskid, msg);
            Some(())
        }
    }
}

mod state {
    use crate::{SessionID, SessionIDRef, TaskId};
    use parking_lot::RwLock;
    use std::collections::BTreeMap;

    type Pool = RwLock<BTreeMap<SessionID, BTreeMap<TaskId, TaskState>>>;
    static STATE_POOL: Pool = StatePool::new();

    #[derive(Debug, Clone)]
    pub enum TaskState {
        NotStarted,
        Running(Vec<String>),
        Completed(Vec<String>),
        Canceled(Vec<String>),
        Error(Vec<String>),
    }

    impl std::fmt::Display for TaskState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    TaskState::NotStarted => "Not Started".to_owned(),
                    TaskState::Running(items) => format!("Running\n{}", items.join("\n")),
                    TaskState::Completed(items) => format!("Completed\n{}", items.join("\n")),
                    TaskState::Canceled(items) => format!("Canceled\n{}", items.join("\n")),
                    TaskState::Error(items) => format!("Error\n{}", items.join("\n")),
                }
            )
        }
    }

    pub enum Reason {
        Start,
        Complete,
        Cancel,
        Error,
    }

    impl TaskState {
        pub fn reason(&mut self, reason: Reason) {
            match reason {
                Reason::Start => {
                    let TaskState::NotStarted = self else {
                        unreachable!()
                    };
                    *self = TaskState::Running(vec![]);
                }
                reason => {
                    let TaskState::Running(items) = self else {
                        unreachable!()
                    };
                    let vec = std::mem::take(items);
                    match reason {
                        Reason::Start => unreachable!(),
                        Reason::Complete => std::mem::replace(self, TaskState::Completed(vec)),
                        Reason::Cancel => std::mem::replace(self, TaskState::Canceled(vec)),
                        Reason::Error => std::mem::replace(self, TaskState::Error(vec)),
                    };
                }
            }
        }
        pub fn update(&mut self, new: String) {
            let TaskState::Running(items) = self else {
                unreachable!()
            };
            items.push(new);
        }
    }

    pub struct StatePool;

    impl StatePool {
        const fn new() -> Pool {
            RwLock::new(BTreeMap::new())
        }

        pub fn new_task(uuid: SessionID, id: TaskId) {
            STATE_POOL
                .write()
                .entry(uuid)
                .or_default()
                .insert(id, TaskState::NotStarted);
        }

        pub fn reason_task(uuid: SessionIDRef, id: TaskId, reason: Reason) {
            STATE_POOL
                .write()
                .get_mut(uuid)
                .map(|map| map.get_mut(&id))
                .flatten()
                .map(|state| state.reason(reason));
        }

        pub fn update_task(uuid: SessionIDRef, id: TaskId, new: String) {
            STATE_POOL
                .write()
                .get_mut(uuid)
                .map(|map| map.get_mut(&id))
                .flatten()
                .map(|state| state.update(new));
        }
    }
}

mod log {
    use crate::{SessionID, SessionIDRef};
    use parking_lot::RwLock;
    use std::collections::BTreeMap;
    use tokio::sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot,
    };

    static LOG_POOL: RwLock<BTreeMap<String, Vec<String>>> = RwLock::new(BTreeMap::new());

    pub fn get_skip_len(uuid: SessionIDRef, len: i32) -> Vec<String> {
        LOG_POOL
            .read()
            .get(uuid)
            .iter()
            .flat_map(|vec| vec.iter())
            .skip(len as usize)
            .cloned()
            .collect()
    }

    /// will be used in callback,
    /// which is out of tokio runtime
    pub static TX_HANDLERS: RwLock<
        BTreeMap<SessionID, crate::log::Logger<String, Result<(), String>>>,
    > = RwLock::new(BTreeMap::new());

    pub struct Logger<T, R> {
        tx: UnboundedSender<T>,
        rx: Option<UnboundedReceiver<T>>,
        oneshot: Option<oneshot::Sender<R>>,
    }

    impl Logger<String, Result<(), String>> {
        pub fn new() -> (Self, oneshot::Receiver<Result<(), String>>) {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let (oneshot_tx, oneshot_rx) = oneshot::channel();
            (
                Self {
                    tx,
                    rx: Some(rx),
                    oneshot: Some(oneshot_tx),
                },
                oneshot_rx,
            )
        }

        pub fn connect_failed(mut self, err: String) {
            if let Some(shot) = self.oneshot.take() {
                let _ = shot.send(Err(err));
            }
        }
        pub fn connect_success(&mut self) {
            if let Some(shot) = self.oneshot.take() {
                let _ = shot.send(Ok(()));
            }
        }

        pub fn take_rx(&mut self) -> Option<UnboundedReceiver<String>> {
            self.rx.take()
        }
        /// if true, the channel is closed, so drop this
        pub fn log(&self, message: String, session_id: SessionID) -> bool {
            Self::log_to_pool(&message, session_id);
            // log to global log pool
            self.tx.send(message).is_err()
        }
        pub fn log_to_pool(message: &str, session_id: SessionID) {
            LOG_POOL
                .write()
                .entry(session_id)
                .or_default()
                .push(message.to_owned());
        }
    }
}

mod task {
    use maa_server::{
        task::{task_server::TaskServer, task_state::State, *},
        tonic::{self, Request, Response},
    };
    use std::collections::BTreeMap;
    use tokio::sync::RwLock;

    use crate::{log::TX_HANDLERS, SessionID};

    /// build service under package task
    ///
    /// ### Note:
    ///
    /// In order to trace and sync client, an additional header `SESSION_KEY` is needed.
    ///
    /// Client get one by calling [`Task::new_connection`], and destory by calling [`Task::close_connection`]
    ///
    /// ### Usage:
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let addr = "[::1]:10000".parse().unwrap();
    ///
    ///     let svc = task::gen_service();
    ///
    ///     Server::builder().add_service(svc).serve(addr).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`Task::new_connection`]: task_server::Task::new_connection
    /// [`Task::close_connection`]: task_server::Task::close_connection
    pub fn gen_service() -> TaskServer<TaskImpl> {
        TaskServer::new(TaskImpl)
    }

    mod wrapper {
        use tokio::sync::Notify;

        use crate::SessionIDRef;

        /// A wrapper for [`maa_sys::Assistant`]
        ///
        /// The inner can be [Send] but not [Sync],
        /// because every fn related is actually a `ref mut` rather `ref`,
        /// which may cause data race
        ///
        /// By using [Notify], only one request can reach handler at a time
        /// and there should be no data racing
        pub struct Assistant {
            inner: maa_sys::Assistant,
            lock: Notify,
        }

        unsafe impl Sync for Assistant {}

        impl Assistant {
            pub fn new(session_id: SessionIDRef) -> Self {
                let instance = Self {
                    inner: maa_sys::Assistant::new(
                        Some(crate::default_callback),
                        Some(
                            std::ffi::CString::new(session_id).unwrap().into_raw() as *mut _
                                as *mut std::ffi::c_void,
                        ),
                    ),
                    lock: Notify::new(),
                };
                instance.lock.notify_one();
                instance
            }

            pub async fn wait(&self) -> &maa_sys::Assistant {
                self.lock.notified().await;
                self.lock.notify_one();
                self.inner_unchecked()
            }

            pub fn inner_unchecked(&self) -> &maa_sys::Assistant {
                &self.inner
            }
        }

        pub trait SessionExt {
            fn get_session_id(&self) -> tonic::Result<SessionIDWrapper>;
        }

        impl<T> SessionExt for tonic::Request<T> {
            fn get_session_id(&self) -> tonic::Result<SessionIDWrapper> {
                self.metadata()
                    .get("x-session-id")
                    .ok_or(tonic::Status::not_found("session_id is not found"))?
                    .to_str()
                    .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
                    .inspect(|session_id| tracing::trace!("Session ID: {session_id}"))
                    .map(SessionIDWrapper)
            }
        }

        impl SessionExt for tonic::metadata::MetadataMap {
            fn get_session_id(&self) -> tonic::Result<SessionIDWrapper> {
                self.get("x-session-id")
                    .ok_or(tonic::Status::not_found("session_id is not found"))?
                    .to_str()
                    .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
                    .inspect(|session_id| tracing::trace!("Session ID: {session_id}"))
                    .map(SessionIDWrapper)
            }
        }

        pub struct SessionIDWrapper<'a>(SessionIDRef<'a>);

        impl<'a> SessionIDWrapper<'a> {
            pub async fn func_with<T>(
                &self,
                f: impl FnOnce(&maa_sys::Assistant) -> T,
            ) -> tonic::Result<T> {
                let read_lock = super::TASK_HANDLERS.read().await;

                let handler = read_lock
                    .get(self.0)
                    .ok_or(tonic::Status::not_found("session_id is not found"))?;

                Ok(f(handler.wait().await))
            }
            pub fn into_inner(self) -> SessionIDRef<'a> {
                self.0
            }
        }
    }

    use wrapper::{Assistant, SessionExt};

    static TASK_HANDLERS: RwLock<BTreeMap<SessionID, Assistant>> =
        RwLock::const_new(BTreeMap::new());

    pub struct TaskImpl;

    type Ret<T> = tonic::Result<Response<T>>;

    #[tonic::async_trait]
    impl task_server::Task for TaskImpl {
        #[tracing::instrument(skip_all)]
        async fn new_connection(&self, req: Request<NewConnectionRequst>) -> Ret<String> {
            let req = req.into_inner();

            let session_id = uuid::Uuid::now_v7().to_string();

            let asst = Assistant::new(&session_id);
            tracing::debug!("Instance Created");

            let (logger, oneshot) = crate::log::Logger::new();
            tracing::debug!("Register C CallBack");
            // ensure we can get callback
            TX_HANDLERS.write().insert(session_id.clone(), logger);

            req.apply_to(&asst.inner_unchecked())?;
            tracing::debug!("Check Connection");
            oneshot
                .await
                .unwrap()
                .map_err(|e| tonic::Status::unavailable(e.to_string()))?;
            tracing::debug!("Register Task State CallBack");
            TASK_HANDLERS.write().await.insert(session_id.clone(), asst);

            Ok(Response::new(session_id))
        }

        #[tracing::instrument(skip_all)]
        async fn close_connection(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?.into_inner();

            Ok(Response::new(
                TASK_HANDLERS.write().await.remove(session_id).is_some()
                    && TX_HANDLERS.write().remove(session_id).is_some(),
            ))
        }

        #[tracing::instrument(skip_all)]
        async fn append_task(&self, req: Request<NewTaskRequest>) -> Ret<TaskId> {
            let (
                meta,
                _,
                NewTaskRequest {
                    task_type,
                    task_params,
                },
            ) = req.into_parts();

            let session_id = meta.get_session_id()?;

            let task_type: TaskType = task_type.try_into().unwrap();
            let task_type: maa_types::TaskType = task_type.into();

            let ret = session_id
                .func_with(|handler| handler.append_task(task_type, task_params.as_str()))
                .await?;

            match ret {
                Ok(id) => {
                    crate::state::StatePool::new_task(session_id.into_inner().to_owned(), id);
                    Ok(Response::new(id.into()))
                }
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn modify_task(&self, req: Request<ModifyTaskRequest>) -> Ret<bool> {
            let (
                meta,
                _,
                ModifyTaskRequest {
                    task_id,
                    task_params,
                },
            ) = req.into_parts();

            let task_id = task_id
                .ok_or(tonic::Status::invalid_argument("no task_id is given"))?
                .into();

            let session_id = meta.get_session_id()?;

            let ret = session_id
                .func_with(|handler| handler.set_task_params(task_id, task_params.as_str()))
                .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn active_task(&self, req: Request<TaskId>) -> Ret<bool> {
            let (meta, _, task_id) = req.into_parts();

            let session_id = meta.get_session_id()?;

            let ret = session_id
                .func_with(|handler| {
                    handler.set_task_params(task_id.into(), r#"{ "enable": true }"#)
                })
                .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn deactive_task(&self, req: Request<TaskId>) -> Ret<bool> {
            let (meta, _, task_id) = req.into_parts();

            let session_id = meta.get_session_id()?;

            let ret = session_id
                .func_with(|handler| {
                    handler.set_task_params(task_id.into(), r#"{ "enable": false }"#)
                })
                .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn start_tasks(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?;

            let ret = session_id.func_with(|handler| handler.start()).await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn stop_tasks(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?;

            let ret = session_id.func_with(|handler| handler.stop()).await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        type TaskStateUpdateStream = std::pin::Pin<
            Box<dyn tokio_stream::Stream<Item = tonic::Result<TaskState>> + Send + 'static>,
        >;

        #[tracing::instrument(skip_all)]
        async fn task_state_update(&self, req: Request<()>) -> Ret<Self::TaskStateUpdateStream> {
            let session_id = req.get_session_id()?.into_inner();

            let Some(rx) = TX_HANDLERS
                .write()
                .get_mut(session_id)
                .and_then(|logger| logger.take_rx())
            else {
                return Err(tonic::Status::resource_exhausted("rx has been taken"));
            };

            use tokio_stream::StreamExt as _;
            let streaming = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(|msg| {
                let state = if !maa_sys::binding::loaded() {
                    State::Unloaded
                } else {
                    State::Idle
                };
                let mut st = TaskState::default();
                st.set_state(state);
                st.content = msg;
                Ok(st)
            });

            Ok(Response::new(Box::pin(streaming)))
        }

        #[tracing::instrument(skip_all)]
        async fn fetch_logs(&self, req: Request<i32>) -> Ret<LogArray> {
            let (meta, _, skip) = req.into_parts();

            let session_id = meta.get_session_id()?.into_inner();

            let logs = crate::log::get_skip_len(session_id, skip);

            Ok(Response::new(LogArray {
                items: logs
                    .into_iter()
                    .map(|log| TaskState {
                        content: log,
                        state: 0,
                    })
                    .collect(),
            }))
        }
    }
}

mod core {
    use maa_server::{
        core::{core_server::CoreServer, *},
        tonic::{self, Request, Response},
    };

    /// build service under package core
    ///
    /// ### Usage:
    /// ```no_run
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let addr = "[::1]:10000".parse().unwrap();
    ///
    ///     let svc = core::gen_service();
    ///
    ///     Server::builder().add_service(svc).serve(addr).await?;
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn gen_service() -> CoreServer<CoreImpl> {
        CoreServer::new(CoreImpl)
    }

    pub struct CoreImpl;

    type Ret<T> = tonic::Result<Response<T>>;

    #[tonic::async_trait]
    impl core_server::Core for CoreImpl {
        #[tracing::instrument(skip_all)]
        async fn load_core(&self, req: Request<CoreConfig>) -> Ret<bool> {
            let core_cfg = req.into_inner();

            if maa_sys::binding::loaded() {
                tracing::debug!("MaaCore already loaded, skiping Core load");
                // using false here to info the client that core is already loaded
                return Ok(Response::new(false));
            }

            maa_server::utils::load_core().map_err(|e| tonic::Status::unknown(e))?;

            core_cfg.apply()?;

            if maa_server::utils::ResourceConfig::default().load().is_err() {
                return Err(tonic::Status::internal("Failed to load resources"));
            }

            Ok(Response::new(true))
        }

        #[tracing::instrument(skip_all)]
        async fn unload_core(&self, _: Request<()>) -> Ret<bool> {
            maa_sys::binding::unload();

            Ok(Response::new(true))
        }
    }
}

use tonic::transport::Server;
use tracing_subscriber::{filter, fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};

// #[tokio::main(flavor = "current_thread")]
#[cfg(feature = "unix-socket")]
#[tokio::main]
async fn main() {
    tracing_subscriber::Registry::default()
        .with(
            fmt::layer()
                .compact()
                .with_ansi(true)
                .with_filter(filter::LevelFilter::DEBUG),
        )
        .init();

    use tokio::net::UnixListener;
    use tokio_stream::wrappers::UnixListenerStream;

    let path = "/tmp/tonic/testing.sock";
    std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();

    let socket = UnixListener::bind(path).unwrap();
    let stream = UnixListenerStream::new(socket);
    Server::builder()
        .add_service(task::gen_service())
        .add_service(core::gen_service())
        .serve_with_incoming(stream)
        .await
        .unwrap();
}

#[cfg(not(feature = "unix-socket"))]
#[tokio::main]
async fn main() {
    tracing_subscriber::Registry::default()
        .with(
            fmt::layer()
                .compact()
                .with_ansi(true)
                .with_filter(filter::LevelFilter::DEBUG),
        )
        .init();

    Server::builder()
        .add_service(task::gen_service())
        .add_service(core::gen_service())
        .serve("127.0.0.1:50051".parse().unwrap())
        .await
        .unwrap();
}

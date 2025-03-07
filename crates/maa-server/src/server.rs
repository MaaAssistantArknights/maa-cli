pub unsafe extern "C" fn default_callback(
    code: maa_types::primitive::AsstMsgId,
    json_str: *const std::ffi::c_char,
    session_id: *mut std::ffi::c_void,
) {
    let code: maa_server::callback::AsstMsg = code.into();
    let json_str = unsafe { std::ffi::CStr::from_ptr(json_str).to_str().unwrap() };
    let session_id: SessionID = unsafe {
        let mut raw = [0u8; 16];
        let ptr = session_id as *mut u8;
        let len = 16;
        raw.copy_from_slice(std::slice::from_raw_parts(ptr, len));
        raw
    };
    callback::main(code, json_str, session_id);
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    #[test]
    fn uuid_ffi() {
        let rust: [u8; 8] = [1, 7, 45, 31, 5, 21, 46, 1];

        let ptr = {
            let mut rust_copy = rust.to_vec();
            let ptr = rust_copy.as_mut_ptr();
            std::mem::forget(rust_copy);
            ptr as *mut std::ffi::c_void
        };
        let len = 8;

        let mut cffi = [0u8; 8];

        assert_ne!(rust, cffi);

        let ptr = ptr as *mut u8;
        cffi.copy_from_slice(unsafe { std::slice::from_raw_parts(ptr, len) });

        assert_eq!(rust, cffi);
    }

    #[test]
    fn uuid_string() {
        let uuid = uuid::Uuid::now_v7();
        let str = uuid.to_string();
        let bytes = uuid.to_bytes_le();

        let bytes_from_str = uuid::Uuid::from_str(&str).unwrap();

        assert_eq!(uuid, bytes_from_str);
        assert_eq!(bytes, bytes_from_str.to_bytes_le());
    }
}

use maa_server::{session::Session, types::*};

mod callback {
    use crate::{Session, SessionID, TaskId};
    use maa_server::callback::AsstMsg;
    use tracing::{debug, error, info, trace, warn};

    type Map = serde_json::Map<String, serde_json::Value>;

    #[tracing::instrument("C CallBack", skip_all)]
    pub fn main(code: maa_server::callback::AsstMsg, json_str: &str, session_id: SessionID) {
        trace!("Session ID: {:?}", session_id);

        Session::log(session_id).log(json_str.to_string());

        let map: Map = serde_json::from_str(json_str).unwrap();

        // if ret is None, which means the message is not processed well
        // we should print the message to trace the error
        if process_message(code, map, session_id).is_none() {
            debug!(
                "FailedToProcessMessage, code: {:?}, message: {}",
                code, json_str
            )
        }
    }

    fn process_message(code: AsstMsg, message: Map, session_id: SessionID) -> Option<()> {
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

    fn process_connection_info(message: Map, session_id: SessionID) -> Option<()> {
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
                Session::test_connection_result(session_id, None);
            }
            "ConnectFailed" => {
                let err = format!("Failed to connect to android device, {}, Please check your connect configuration: {}",
                    why.unwrap(),serde_json::to_string_pretty(&details).unwrap());
                error!(err);
                Session::test_connection_result(session_id, Some(err));
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
                "{}: what:{} why:{} details:{}",
                "Unknown Connection Info",
                what,
                why.as_deref().unwrap_or("No why"),
                serde_json::to_string_pretty(&details).unwrap()
            ),
        }

        Some(())
    }

    fn process_taskchain(code: AsstMsg, message: Map, session_id: SessionID) -> Option<()> {
        #[derive(serde::Deserialize)]
        struct TaskChain {
            taskchain: maa_types::TaskType,
            taskid: TaskId,
        }
        let TaskChain { taskchain, taskid } =
            serde_json::from_value(serde_json::Value::Object(message)).unwrap();

        use maa_server::session::{Session, State};
        use AsstMsg::*;

        match code {
            TaskChainStart => {
                info!("{} {}", taskchain, "Start");
                Session::tasks(session_id).state(taskid, State::Running);
            }
            TaskChainCompleted => {
                info!("{} {}", taskchain, "Completed");
                Session::tasks(session_id).state(taskid, State::Completed);
            }
            TaskChainStopped => {
                warn!("{} {}", taskchain, "Stopped");
                Session::tasks(session_id).state(taskid, State::Canceled);
            }
            TaskChainError => {
                error!("{} {}", taskchain, "Error");
                Session::tasks(session_id).state(taskid, State::Error);
            }
            TaskChainExtraInfo => {}

            _ => {} // unreachable
        };

        Some(())
    }

    mod subtask {
        use super::*;

        pub fn process_subtask(_code: AsstMsg, message: Map, session_id: SessionID) -> Option<()> {
            let msg = serde_json::to_string_pretty(&message).unwrap();
            let taskid = message.get("taskid")?.as_i64()? as TaskId;
            Session::tasks(session_id).update(taskid, msg);
            Some(())
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

    use crate::{Session, SessionID};

    /// build service under package task
    ///
    /// ### Note:
    ///
    /// In order to trace and sync client, an additional header `SESSION_KEY` is needed.
    ///
    /// Client get one by calling [`Task::new_connection`], and destroy by calling [`Task::close_connection`]
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
        use std::str::FromStr;

        use tokio::sync::Notify;

        use crate::SessionID;

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
            pub fn new(session_id: SessionID) -> Self {
                // this Vec created is used to forget
                // otherwise the raw content will be dropped
                // and callback will get an different SessionID
                // which is dangerous
                let mut session_id = session_id.to_vec();
                let ptr = session_id.as_mut_ptr();
                std::mem::forget(session_id);
                let instance = Self {
                    inner: maa_sys::Assistant::new(
                        Some(crate::default_callback),
                        Some(ptr as *mut std::ffi::c_void),
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
            fn get_session_id(&self) -> tonic::Result<SessionID>;
        }

        impl<T> SessionExt for tonic::Request<T> {
            fn get_session_id(&self) -> tonic::Result<SessionID> {
                self.metadata().get_session_id()
            }
        }

        impl SessionExt for tonic::metadata::MetadataMap {
            fn get_session_id(&self) -> tonic::Result<SessionID> {
                self.get("x-session-id")
                    .ok_or(tonic::Status::not_found("session_id is not found"))?
                    .to_str()
                    .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
                    .and_then(|str| {
                        uuid::Uuid::from_str(str)
                            .map_err(|_| tonic::Status::invalid_argument("session_id is not valid"))
                    })
                    .map(|uuid| uuid.to_bytes_le())
            }
        }

        pub async fn func_with<T>(
            session_id: SessionID,
            f: impl FnOnce(&maa_sys::Assistant) -> T,
        ) -> tonic::Result<T> {
            let read_lock = super::TASK_HANDLERS.read().await;

            let handler = read_lock
                .get(&session_id)
                .ok_or(tonic::Status::not_found("session_id is not found"))?;

            Ok(f(handler.wait().await))
        }
    }

    use wrapper::{func_with, Assistant, SessionExt};

    static TASK_HANDLERS: RwLock<BTreeMap<SessionID, Assistant>> =
        RwLock::const_new(BTreeMap::new());

    pub struct TaskImpl;

    type Ret<T> = tonic::Result<Response<T>>;

    #[tonic::async_trait]
    impl task_server::Task for TaskImpl {
        #[tracing::instrument(skip_all)]
        async fn new_connection(&self, req: Request<NewConnectionRequest>) -> Ret<String> {
            let req = req.into_inner();

            let raw_session_id = uuid::Uuid::now_v7();
            let session_id = raw_session_id.to_bytes_le();

            let asst = Assistant::new(session_id);
            tracing::debug!("Instance Created");

            tracing::debug!("Register C CallBack");
            let (tx, rx) = tokio::sync::oneshot::channel();
            Session::new(session_id, tx);

            req.apply_to(asst.inner_unchecked())?;
            tracing::debug!("Check Connection");
            rx.await
                .unwrap()
                .map_err(|e| tonic::Status::unavailable(e.to_string()))?;

            tracing::debug!("Register Task State CallBack");
            TASK_HANDLERS.write().await.insert(session_id, asst);

            Ok(Response::new(raw_session_id.to_string()))
        }

        #[tracing::instrument(skip_all)]
        async fn close_connection(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?;

            Ok(Response::new(
                TASK_HANDLERS.write().await.remove(&session_id).is_some()
                    && Session::remove(session_id),
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

            let ret = func_with(session_id, |handler| {
                handler.append_task(task_type, task_params.as_str())
            })
            .await?;

            match ret {
                Ok(id) => {
                    Session::tasks(session_id).new(id);
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

            let ret = func_with(session_id, |handler| {
                handler.set_task_params(task_id, task_params.as_str())
            })
            .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn activate_task(&self, req: Request<TaskId>) -> Ret<bool> {
            let (meta, _, task_id) = req.into_parts();

            let session_id = meta.get_session_id()?;

            let ret = func_with(session_id, |handler| {
                handler.set_task_params(task_id.into(), r#"{ "enable": true }"#)
            })
            .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn deactivate_task(&self, req: Request<TaskId>) -> Ret<bool> {
            let (meta, _, task_id) = req.into_parts();

            let session_id = meta.get_session_id()?;

            let ret = func_with(session_id, |handler| {
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

            let ret = func_with(session_id, |handler| handler.start()).await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        #[tracing::instrument(skip_all)]
        async fn stop_tasks(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?;

            let ret = func_with(session_id, |handler| handler.stop()).await?;

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
            let session_id = req.get_session_id()?;

            let Some(rx) = Session::take_subscriber(session_id) else {
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

            let session_id = meta.get_session_id()?;

            let logs = Session::log(session_id).get_skip(skip);

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
                tracing::debug!("MaaCore already loaded, skipping Core load");
                // using false here to info the client that core is already loaded
                return Ok(Response::new(false));
            }

            maa_server::utils::load_core().map_err(tonic::Status::unknown)?;

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

    #[cfg(not(feature = "unix-socket"))]
    let stream = {
        tokio_stream::wrappers::TcpListenerStream::new(
            tokio::net::TcpListener::bind("127.0.0.1:50051")
                .await
                .unwrap(),
        )
    };

    #[cfg(feature = "unix-socket")]
    let stream = {
        let path = "/tmp/tonic/testing.sock";
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        tokio_stream::wrappers::UnixListenerStream::new(
            tokio::net::UnixListener::bind(path).unwrap(),
        )
    };

    Server::builder()
        .add_service(task::gen_service())
        .add_service(core::gen_service())
        .serve_with_incoming(stream)
        .await
        .unwrap();
}

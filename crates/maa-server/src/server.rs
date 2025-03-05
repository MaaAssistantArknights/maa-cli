pub unsafe extern "C" fn default_callback(
    code: maa_types::primitive::AsstMsgId,
    json_raw: *const ::std::os::raw::c_char,
    _: *mut ::std::os::raw::c_void,
) {
    use log::Logger;
    let _ = code;
    let json_str = unsafe { std::ffi::CStr::from_ptr(json_raw).to_str().unwrap() };

    let uuid = Logger::uuid(json_str);
    if let Some(tx) = log::TX_HANDLERS.read().get(&uuid) {
        if tx.log(json_str.to_string()) {
            log::TX_HANDLERS.write().remove(&uuid);
        }
    } else {
        Logger::log_to_pool(json_str);
    }
}

type UUID = String;

mod log {
    use crate::UUID;
    use parking_lot::RwLock;
    use std::collections::BTreeMap;
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

    static LOG_POOL: RwLock<BTreeMap<String, Vec<String>>> = RwLock::new(BTreeMap::new());

    pub fn get_skip_len(uuid: &str, len: i32) -> Vec<String> {
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
    pub static TX_HANDLERS: RwLock<BTreeMap<UUID, crate::log::Logger<String>>> =
        RwLock::new(BTreeMap::new());

    pub struct Logger<T> {
        tx: UnboundedSender<T>,
        rx: Option<UnboundedReceiver<T>>,
    }

    impl Logger<String> {
        pub fn new() -> Self {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            Self { tx, rx: Some(rx) }
        }
        pub fn take_rx(&mut self) -> Option<UnboundedReceiver<String>> {
            self.rx.take()
        }
        /// if true, the channel is closed, so drop this
        pub fn log(&self, message: String) -> bool {
            Self::log_to_pool(&message);
            // log to global log pool
            self.tx.send(message).is_err()
        }
        pub fn uuid(message: &str) -> UUID {
            #[derive(serde::Deserialize)]
            struct DeSer {
                uuid: UUID,
                #[serde(flatten)]
                __extra: BTreeMap<String, serde_json::Value>,
            }
            let value: DeSer = serde_json::from_str(&message).unwrap_or(DeSer {
                uuid: "()".to_owned(),
                __extra: Default::default(),
            });
            value.uuid
        }
        pub fn log_to_pool(message: &str) {
            let uuid = Self::uuid(message);
            LOG_POOL
                .write()
                .entry(uuid)
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

    use crate::{log::TX_HANDLERS, UUID};

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
            pub fn new() -> Self {
                let instance = Self {
                    inner: maa_sys::Assistant::new(Some(crate::default_callback), None),
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
            fn get_session_id(&self) -> tonic::Result<UUIDWrapper>;
        }

        impl<T> SessionExt for tonic::Request<T> {
            fn get_session_id(&self) -> tonic::Result<UUIDWrapper> {
                self.metadata()
                    .get("x-session-key")
                    .ok_or(tonic::Status::not_found("session_id is not found"))?
                    .to_str()
                    .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
                    .map(UUIDWrapper)
            }
        }

        impl SessionExt for tonic::metadata::MetadataMap {
            fn get_session_id(&self) -> tonic::Result<UUIDWrapper> {
                self.get("x-session-key")
                    .ok_or(tonic::Status::not_found("session_id is not found"))?
                    .to_str()
                    .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
                    .map(UUIDWrapper)
            }
        }

        pub struct UUIDWrapper<'a>(&'a str);

        impl<'a> UUIDWrapper<'a> {
            pub async fn func_with<T>(
                self,
                f: impl FnOnce(&maa_sys::Assistant) -> T,
            ) -> tonic::Result<T> {
                let read_lock = super::TASK_HANDLERS.read().await;

                let handler = read_lock
                    .get(self.0)
                    .ok_or(tonic::Status::not_found("session_id is not found"))?;

                Ok(f(handler.wait().await))
            }
            pub fn into_inner(self) -> &'a str {
                self.0
            }
        }
    }

    use wrapper::{Assistant, SessionExt};

    static TASK_HANDLERS: RwLock<BTreeMap<UUID, Assistant>> = RwLock::const_new(BTreeMap::new());

    pub struct TaskImpl;

    type Ret<T> = tonic::Result<Response<T>>;

    #[tonic::async_trait]
    impl task_server::Task for TaskImpl {
        async fn new_connection(&self, req: Request<NewConnectionRequst>) -> Ret<String> {
            let NewConnectionRequst { conncfg, instcfg } = req.into_inner();

            let asst = Assistant::new();

            if let Some(message) =
                instcfg.and_then(|cfg| cfg.apply_to(asst.inner_unchecked()).err())
            {
                return Err(tonic::Status::internal(message));
            }

            let (adb_path, address, config) = conncfg.unwrap().connect_args();
            asst.inner_unchecked()
                .async_connect(adb_path.as_str(), address.as_str(), config.as_str(), true)
                .unwrap();

            let session_id = asst.inner_unchecked().get_uuid_ext();

            TX_HANDLERS
                .write()
                .insert(session_id.clone(), crate::log::Logger::new());
            TASK_HANDLERS.write().await.insert(session_id.clone(), asst);

            Ok(Response::new(session_id))
        }

        async fn close_connection(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?.into_inner();

            Ok(Response::new(
                TASK_HANDLERS.write().await.remove(session_id).is_some()
                    && TX_HANDLERS.write().remove(session_id).is_some(),
            ))
        }

        async fn append_task(&self, new_task: Request<NewTaskRequest>) -> Ret<TaskId> {
            let (
                meta,
                _,
                NewTaskRequest {
                    task_type,
                    task_params,
                },
            ) = new_task.into_parts();

            let session_id = meta.get_session_id()?;

            let task_type: TaskType = task_type.try_into().unwrap();
            let task_type: maa_types::TaskType = task_type.into();

            let ret = session_id
                .func_with(|handler| handler.append_task(task_type, task_params.as_str()))
                .await?;

            match ret {
                Ok(id) => Ok(Response::new(id.into())),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        async fn modify_task(&self, task_param: Request<ModifyTaskRequest>) -> Ret<bool> {
            let (
                meta,
                _,
                ModifyTaskRequest {
                    task_id,
                    task_params,
                },
            ) = task_param.into_parts();

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

        async fn active_task(&self, task_id: Request<TaskId>) -> Ret<bool> {
            let (meta, _, task_id) = task_id.into_parts();

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

        async fn deactive_task(&self, task_id: Request<TaskId>) -> Ret<bool> {
            let (meta, _, task_id) = task_id.into_parts();

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

        async fn start_tasks(&self, req: Request<()>) -> Ret<bool> {
            let session_id = req.get_session_id()?;

            let ret = session_id.func_with(|handler| handler.start()).await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

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
        async fn load_core(&self, req: Request<CoreConfig>) -> Ret<bool> {
            let CoreConfig {
                static_ops,
                log_ops,
            } = req.into_inner();

            let tmp = tracing::span!(tracing::Level::DEBUG, "");
            let _enter = tmp.enter();

            let ret = maa_server::utils::load_core();

            if let Some(core_config::LogOptions { level, name }) = log_ops {
                use maa_dirs::Ensure;
                // Todo: set log level for tracing
                let _ = level;
                maa_sys::Assistant::set_user_dir(
                    maa_dirs::state()
                        .join(name)
                        .as_path()
                        .ensure()
                        .map_err(|e| tonic::Status::from_error(Box::new(e)))?,
                )
                .unwrap();
            }

            if let Some(core_config::StaticOptions { cpu_ocr, gpu_ocr }) = static_ops {
                use maa_sys::{Assistant, StaticOptionKey};
                match (cpu_ocr, gpu_ocr) {
                    (cpu_ocr, Some(gpu_id)) => {
                        if cpu_ocr {
                            tracing::warn!(
                                "Both CPU OCR and GPU OCR are enabled, CPU OCR will be ignored"
                            );
                        }
                        tracing::debug!("Using GPU OCR with GPU ID {}", gpu_id);
                        if Assistant::set_static_option(StaticOptionKey::GpuOCR, gpu_id).is_err() {
                            return Err(tonic::Status::internal(format!(
                                "Failed to enable GPU OCR with GPU ID {}",
                                gpu_id
                            )));
                        }
                    }
                    (true, None) => {
                        tracing::debug!("Using CPU OCR");
                        if Assistant::set_static_option(StaticOptionKey::CpuOCR, true).is_err() {
                            return Err(tonic::Status::internal("Failed to enable CPU OCR"));
                        }
                    }
                    (false, None) => {}
                }
            }

            if maa_server::utils::ResourceConfig::default().load().is_err() {
                return Err(tonic::Status::internal("Failed to load resources"));
            }

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::unknown(e)),
            }
        }
        async fn unload_core(&self, _: Request<()>) -> Ret<bool> {
            maa_sys::binding::unload();

            Ok(Response::new(true))
        }
    }
}

use tonic::transport::Server;

// #[tokio::main(flavor = "current_thread")]
#[tokio::main]
async fn main() {
    let using_socket = true;

    tracing_subscriber::fmt::init();

    if using_socket {
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
    } else {
        Server::builder()
            .add_service(task::gen_service())
            .add_service(core::gen_service())
            .serve("127.0.0.1:50051".parse().unwrap())
            .await
            .unwrap();
    }
}

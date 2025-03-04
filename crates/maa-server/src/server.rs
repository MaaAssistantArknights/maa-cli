pub unsafe extern "C" fn default_callback(
    code: maa_types::primitive::AsstMsgId,
    json_raw: *const ::std::os::raw::c_char,
    _: *mut ::std::os::raw::c_void,
) {
    let _ = code;
    let json_str = unsafe { std::ffi::CStr::from_ptr(json_raw).to_str().unwrap() };
    if let Some(tx) = task::TX_HANDLERS.read().unwrap().get("62e47172a08776c8") {
        // ignore this here, because client might exit without call close_connection
        // which will cause panic here due to the dropped Receiver
        let _ = tx.send(json_str.to_string());
    } else {
        // some content -- or let's be clear, adb connection info will not be able to transfer
        // to client, because the tx is not created until task_state_update, which is always 
        // after new_connection since the session_id is required!
        
        // This might cannot be fixed -- the uuid is known only after connected to the device
        // However, a new channel that send info to new_connection might help
        println!("{}", json_str);
    }
}

mod task {
    use maa_server::{
        task::{task_server::TaskServer, task_state::State, *},
        tonic::{self, metadata::MetadataMap, Request, Response},
    };
    use std::collections::BTreeMap;
    use tokio::sync::{Notify, RwLock};
    use tokio_stream::Stream;

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

    /// A wrapper for [`maa_sys::Assistant`]
    ///
    /// The inner can be [Send] but not [Sync],
    /// because every fn related is actually a `ref mut` rather `ref`,
    /// which may cause data race
    ///
    /// By using [Notify], only one request can reach handler at a time
    /// and there should be no data racing
    struct Assistant {
        inner: maa_sys::Assistant,
        lock: Notify,
    }

    unsafe impl Sync for Assistant {}

    impl Assistant {
        pub fn new() -> Self {
            let instance = Self {
                inner: maa_sys::Assistant::new(Some(super::default_callback), None),
                lock: Notify::new(),
            };
            instance.lock.notify_one();
            instance
        }

        pub async fn wait(&self) -> &maa_sys::Assistant {
            self.lock.notified().await;
            self.lock.notify_one();
            &self.inner
        }
    }

    /// Cannot use async version
    /// since callback is not in the runtime
    ///
    /// Given that we `write` this only when [task_server::Task::task_state_update] is called,
    /// a sync version shouldn't block too long
    ///
    /// not tested under `current_thread` mode
    pub static TX_HANDLERS: std::sync::RwLock<
        BTreeMap<String, tokio::sync::mpsc::UnboundedSender<String>>,
    > = std::sync::RwLock::new(BTreeMap::new());
    static TASK_HANDLERS: RwLock<BTreeMap<String, Assistant>> = RwLock::const_new(BTreeMap::new());

    fn get_session_id<'a>(meta: &'a MetadataMap) -> tonic::Result<&'a str> {
        meta.get("x-session-key")
            .ok_or(tonic::Status::not_found("session_id is not found"))?
            .to_str()
            .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
    }

    async fn fun_task_handler<T>(
        session_id: &str,
        f: impl FnOnce(&maa_sys::Assistant) -> T,
    ) -> tonic::Result<T> {
        let read_lock = TASK_HANDLERS.read().await;

        let handler = read_lock
            .get(session_id)
            .ok_or(tonic::Status::not_found("session_id is not found"))?;

        Ok(f(handler.wait().await))
    }

    pub struct TaskImpl;

    type Ret<T> = tonic::Result<Response<T>>;

    #[tonic::async_trait]
    impl task_server::Task for TaskImpl {
        async fn new_connection(&self, req: Request<NewConnectionRequst>) -> Ret<String> {
            let NewConnectionRequst { conncfg, instcfg } = req.into_inner();

            let asst = Assistant::new();

            if let Some(message) = instcfg.and_then(|cfg| cfg.apply_to(&asst.inner).err()) {
                return Err(tonic::Status::internal(message));
            }
            let (adb_path, address, config) = conncfg.unwrap().connect_args();
            asst.inner
                .async_connect(adb_path.as_str(), address.as_str(), config.as_str(), true)
                .unwrap();

            let uuid = unsafe {
                let mut buff_size = 1024;
                loop {
                    if buff_size > 1024 * 1024 {
                        unreachable!();
                    }
                    let mut buff: Vec<u8> = Vec::with_capacity(buff_size);
                    let data_size = asst
                        .inner
                        .get_uuid(buff.as_mut_slice(), buff_size as u64)
                        .unwrap();
                    if data_size == maa_sys::binding::AsstGetNullSize() {
                        buff_size = 2 * buff_size;
                        continue;
                    }
                    buff.set_len(data_size as usize);
                    break String::from_utf8_lossy(&buff).to_string();
                }
            };
            println!(">> {}", uuid);
            let session_id = uuid;
            let mut lock = TASK_HANDLERS.write().await;
            lock.insert(session_id.clone(), asst);
            Ok(Response::new(session_id))
        }

        async fn close_connection(&self, req: Request<()>) -> Ret<bool> {
            let (meta, _, ()) = req.into_parts();

            let session_id = get_session_id(&meta)?;

            Ok(Response::new(
                TASK_HANDLERS.write().await.remove(session_id).is_some()
                    && TX_HANDLERS.write().unwrap().remove(session_id).is_some(),
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

            let session_id = get_session_id(&meta)?;

            let task_type: TaskType = task_type.try_into().unwrap();
            let task_type: maa_types::TaskType = task_type.into();

            let ret = fun_task_handler(session_id, |handler| {
                handler.append_task(task_type, task_params.as_str())
            })
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

            let session_id = get_session_id(&meta)?;

            let ret = fun_task_handler(session_id, |handler| {
                handler.set_task_params(task_id, task_params.as_str())
            })
            .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        async fn active_task(&self, task_id: Request<TaskId>) -> Ret<bool> {
            let (meta, _, task_id) = task_id.into_parts();

            let session_id = get_session_id(&meta)?;

            let ret = fun_task_handler(session_id, |handler| {
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

            let session_id = get_session_id(&meta)?;

            let ret = fun_task_handler(session_id, |handler| {
                handler.set_task_params(task_id.into(), r#"{ "enable": false }"#)
            })
            .await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        async fn start_tasks(&self, req: Request<()>) -> Ret<bool> {
            let (meta, _, ()) = req.into_parts();

            let session_id = get_session_id(&meta)?;

            let ret = fun_task_handler(session_id, |handler| handler.start()).await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        async fn stop_tasks(&self, req: Request<()>) -> Ret<bool> {
            let (meta, _, ()) = req.into_parts();

            let session_id = get_session_id(&meta)?;

            let ret = fun_task_handler(session_id, |handler| handler.stop()).await?;

            match ret {
                Ok(()) => Ok(Response::new(true)),
                Err(e) => Err(tonic::Status::from_error(Box::new(e))),
            }
        }

        type TaskStateUpdateStream =
            std::pin::Pin<Box<dyn Stream<Item = tonic::Result<TaskState>> + Send + 'static>>;

        async fn task_state_update(&self, req: Request<()>) -> Ret<Self::TaskStateUpdateStream> {
            let (meta, _, ()) = req.into_parts();

            let session_id = get_session_id(&meta)?;

            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

            TX_HANDLERS
                .write()
                .unwrap()
                .insert(session_id.to_owned(), tx);

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
            let CoreConfig { static_ops } = req.into_inner();

            let tmp = tracing::span!(tracing::Level::DEBUG, "");
            let _enter = tmp.enter();

            let ret = maa_server::utils::load_core();

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

            unsafe {
                use maa_sys::ToCString;
                maa_sys::binding::AsstSetUserDir(maa_dirs::state().to_cstring().unwrap().as_ptr())
            };

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
        async fn set_log(&self, request: Request<LogRequest>) -> Ret<bool> {
            let _ = request;
            todo!()
        }
    }
}

use tonic::transport::Server;
use tracing_subscriber::util::SubscriberInitExt;

// #[tokio::main(flavor = "current_thread")]
#[tokio::main]
async fn main() {
    tracing_subscriber::registry().init();
    Server::builder()
        .add_service(task::gen_service())
        .add_service(core::gen_service())
        .serve("127.0.0.1:50051".parse().unwrap())
        .await
        .unwrap();
}

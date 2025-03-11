use std::collections::BTreeMap;

use tokio::sync::RwLock;
use tonic::{self, Request, Response};

use crate::{
    session::SessionExt,
    task::{task_server::TaskServer, *},
    types::SessionID,
};

/// build service under package task
///
/// ### Note:
///
/// In order to trace and sync client, an additional header `SESSION_KEY` is needed.
///
/// Client get one by calling [`Task::new_connection`], and destroy by calling
/// [`Task::close_connection`]
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

    use super::SessionID;

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
            let ptr = session_id.to_ptr();
            let instance = Self {
                inner: maa_sys::Assistant::new(
                    Some(crate::server_impl::default_callback),
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

    pub trait SessionIDExt {
        fn get_session_id(&self) -> tonic::Result<SessionID>;
    }

    impl<T> SessionIDExt for tonic::Request<T> {
        fn get_session_id(&self) -> tonic::Result<SessionID> {
            self.metadata().get_session_id()
        }
    }

    impl SessionIDExt for tonic::metadata::MetadataMap {
        fn get_session_id(&self) -> tonic::Result<SessionID> {
            self.get("x-session-id")
                .ok_or(tonic::Status::not_found("session_id is not found"))?
                .to_str()
                .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
                .and_then(|str| {
                    SessionID::from_str(str)
                        .map_err(|_| tonic::Status::invalid_argument("session_id is not valid"))
                })
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

use wrapper::{func_with, Assistant, SessionIDExt};

static TASK_HANDLERS: RwLock<BTreeMap<SessionID, Assistant>> = RwLock::const_new(BTreeMap::new());

pub struct TaskImpl;

type Ret<T> = tonic::Result<Response<T>>;

#[tonic::async_trait]
impl task_server::Task for TaskImpl {
    type TaskStateUpdateStream = std::pin::Pin<
        Box<dyn tokio_stream::Stream<Item = tonic::Result<TaskState>> + Send + 'static>,
    >;

    #[tracing::instrument(skip_all)]
    async fn new_connection(&self, req: Request<NewConnectionRequest>) -> Ret<String> {
        let req = req.into_inner();

        let session_id = SessionID::new();

        let asst = Assistant::new(session_id);
        tracing::debug!("Instance Created");

        tracing::debug!("Register C CallBack");
        let (tx, rx) = tokio::sync::oneshot::channel();
        session_id.add(tx);

        req.apply_to(asst.inner_unchecked())?;
        tracing::debug!("Check Connection");
        rx.await
            .unwrap()
            .map_err(|e| tonic::Status::unavailable(e.to_string()))?;

        tracing::debug!("Register Task State CallBack");
        TASK_HANDLERS.write().await.insert(session_id, asst);

        Ok(Response::new(session_id.to_string()))
    }

    #[tracing::instrument(skip_all)]
    async fn close_connection(&self, req: Request<()>) -> Ret<bool> {
        let session_id = req.get_session_id()?;

        Ok(Response::new(
            TASK_HANDLERS.write().await.remove(&session_id).is_some(),
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

        let ret = func_with(session_id, |handler| {
            handler.append_task(task_type, task_params.as_str())
        })
        .await?;

        match ret {
            Ok(id) => {
                session_id.tasks().new(id);
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

    #[tracing::instrument(skip_all)]
    async fn task_state_update(&self, req: Request<()>) -> Ret<Self::TaskStateUpdateStream> {
        let session_id = req.get_session_id()?;

        let Some(rx) = session_id.take_subscriber() else {
            return Err(tonic::Status::resource_exhausted("rx has been taken"));
        };

        use tokio_stream::StreamExt as _;
        let streaming =
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(|(state, log)| {
                Ok(TaskState {
                    content: log,
                    state: state.into(),
                })
            });

        Ok(Response::new(Box::pin(streaming)))
    }

    #[tracing::instrument(skip_all)]
    async fn fetch_logs(&self, req: Request<i32>) -> Ret<LogArray> {
        let (meta, _, skip) = req.into_parts();

        let session_id = meta.get_session_id()?;

        let logs = session_id.log().get_skip(skip);

        Ok(Response::new(LogArray {
            items: logs
                .into_iter()
                .map(|(state, log)| TaskState {
                    content: log,
                    state: state.into(),
                })
                .collect(),
        }))
    }
}

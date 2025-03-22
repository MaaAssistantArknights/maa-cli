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
/// use maa_server::prelude::task_service;
/// use tokio_util::sync::CancellationToken;
/// use tonic::transport::Server;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let addr = "[::1]:10000".parse().unwrap();
///
///     let svc = task_service();
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
        use std::str::FromStr;
        self.get(crate::types::HEADER_SESSION_ID)
            .ok_or(tonic::Status::not_found("session_id is not found"))?
            .to_str()
            .map_err(|_| tonic::Status::invalid_argument("session_id should be ascii"))
            .and_then(|str| {
                SessionID::from_str(str)
                    .map_err(|_| tonic::Status::invalid_argument("session_id is not valid"))
            })
    }
}

pub struct TaskImpl;

type Ret<T> = tonic::Result<Response<T>>;

#[tonic::async_trait]
impl task_server::Task for TaskImpl {
    type TaskStateUpdateStream = std::pin::Pin<
        Box<dyn tokio_stream::Stream<Item = tonic::Result<TaskState>> + Send + 'static>,
    >;

    #[tracing::instrument(skip_all)]
    async fn test_connection(&self, _req: Request<()>) -> Ret<String> {
        let session_id = SessionID::new();

        let asst = crate::session::Assistant::new(session_id);
        tracing::debug!("Test Instance Created, SessionID: {}", session_id);
        session_id.add(asst);

        Ok(Response::new(session_id.to_string()))
    }

    #[tracing::instrument(skip_all)]
    async fn new_connection(&self, req: Request<NewConnectionRequest>) -> Ret<String> {
        let req = req.into_inner();

        let session_id = SessionID::new();

        let asst = crate::session::Assistant::new(session_id);
        tracing::debug!("Instance Created, SessionID: {}", session_id);

        let (tx, rx) = tokio::sync::oneshot::channel();
        session_id.adb().register(tx);
        tracing::trace!("Register C CallBack");

        req.apply_to(asst.inner_unchecked())?;
        session_id.add(asst);

        tracing::trace!("Check Connection");
        rx.await
            .unwrap()
            .map_err(|e| tonic::Status::failed_precondition(e.to_string()))?;

        Ok(Response::new(session_id.to_string()))
    }

    #[tracing::instrument(skip_all)]
    async fn close_connection(&self, req: Request<()>) -> Ret<bool> {
        let session_id = req.get_session_id()?;

        Ok(Response::new(session_id.remove()))
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

        let ret = session_id.tasks().append(task_type, task_params.as_str());

        match ret {
            Ok(id) => Ok(Response::new(id.into())),
            Err(e) => Err(tonic::Status::from_error(e)),
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

        let ret = session_id.tasks().patch_params(task_id, &task_params);

        match ret {
            Ok(()) => Ok(Response::new(true)),
            Err(e) => Err(tonic::Status::from_error(e)),
        }
    }

    #[tracing::instrument(skip_all)]
    async fn deactivate_task(&self, req: Request<TaskId>) -> Ret<bool> {
        let (meta, _, task_id) = req.into_parts();

        let session_id = meta.get_session_id()?;

        let ret = session_id
            .tasks()
            .patch_params(task_id.into(), r#"{ "enable": false }"#);

        match ret {
            Ok(()) => Ok(Response::new(true)),
            Err(e) => Err(tonic::Status::from_error(e)),
        }
    }

    #[tracing::instrument(skip_all)]
    async fn start_tasks(&self, req: Request<()>) -> Ret<bool> {
        let session_id = req.get_session_id()?;

        let ret = session_id.tasks().start();

        match ret {
            Ok(()) => Ok(Response::new(true)),
            Err(e) => Err(tonic::Status::from_error(e)),
        }
    }

    #[tracing::instrument(skip_all)]
    async fn stop_tasks(&self, req: Request<()>) -> Ret<bool> {
        let session_id = req.get_session_id()?;

        let ret = session_id.tasks().stop();

        match ret {
            Ok(()) => Ok(Response::new(true)),
            Err(e) => Err(tonic::Status::from_error(e)),
        }
    }

    #[tracing::instrument(skip_all)]
    async fn task_state_update(&self, req: Request<()>) -> Ret<Self::TaskStateUpdateStream> {
        let session_id = req.get_session_id()?;

        let Some(rx) = session_id.log().take_subscriber() else {
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

        let logs = session_id.log().get_skip(skip as usize);

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

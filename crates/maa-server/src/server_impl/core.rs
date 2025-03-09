use tokio_util::sync::CancellationToken;
use tonic::{self, Request, Response};

use crate::core::{core_server::CoreServer, *};

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
pub fn gen_service(cancel_token: CancellationToken) -> CoreServer<CoreImpl> {
    CoreServer::new(CoreImpl { cancel_token })
}

pub struct CoreImpl {
    cancel_token: CancellationToken,
}

type Ret<T> = tonic::Result<Response<T>>;

#[tonic::async_trait]
impl core_server::Core for CoreImpl {
    #[tracing::instrument(skip_all)]
    async fn load_core(&self, req: Request<CoreConfig>) -> Ret<bool> {
        let core_cfg = req.into_inner();

        if maa_sys::Assistant::loaded() {
            tracing::debug!("MaaCore already loaded, skipping Core load");
            // using false here to info the client that core is already loaded
            return Ok(Response::new(false));
        }

        core_cfg.apply()?;

        Ok(Response::new(true))
    }

    #[tracing::instrument(skip_all)]
    async fn unload_core(&self, _: Request<()>) -> Ret<bool> {
        maa_sys::Assistant::unload().map_err(|e| tonic::Status::internal(e.to_string()))?;

        tracing::info!("Unload Core");
        self.cancel_token.cancel();

        Ok(Response::new(true))
    }
}


use crate::{
    core::{core_server::CoreServer, *},
    tonic::{self, Request, Response}, utils,
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

        utils::load_core().map_err(tonic::Status::unknown)?;

        core_cfg.apply()?;

        if utils::ResourceConfig::default().load().is_err() {
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
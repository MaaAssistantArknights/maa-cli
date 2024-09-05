#[async_trait::async_trait]
pub(super) trait ExternalApp {
    async fn open(&self) -> anyhow::Result<()>;

    async fn close(&self) -> anyhow::Result<()>;
}

#[cfg(target_os = "macos")]
mod playcover;
#[cfg(target_os = "macos")]
pub(super) use playcover::PlayCoverApp;

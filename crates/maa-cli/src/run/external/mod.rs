pub(super) trait ExternalApp {
    fn open(&self, start_if_needed: bool) -> anyhow::Result<Option<String>>;

    fn close(&self) -> anyhow::Result<()>;
}

#[cfg(target_os = "macos")]
mod playcover;
#[cfg(target_os = "macos")]
pub(super) use playcover::PlayCoverApp;

#[cfg(target_os = "linux")]
mod waydroid;
#[cfg(target_os = "linux")]
pub(super) use waydroid::WaydroidApp;

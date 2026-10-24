use std::path::Path;

use anyhow::{Context, Result};

use super::{
    ConnectionArgs, callback::MaaCallback, create_assistant, load_config, with_connection,
};
use crate::config::task::ClientType;

pub fn screenshot(output: &Path, args: ConnectionArgs) -> Result<()> {
    let config = load_config(&args)?;
    let (callback, _) = MaaCallback::new(false);
    let asst = create_assistant(&config, callback)?;

    with_connection(&asst, &config, ClientType::default(), false, false, || {
        let image = asst
            .get_fresh_image()
            .context("Failed to take screenshot")?;
        save_image(output, image.as_deref())
    })
}

fn save_image(output: &Path, image: Option<&[u8]>) -> Result<()> {
    let image = image.context("No screenshot returned by MaaCore")?;
    maa_atomic_fs::write(output, image)
        .with_context(|| format!("Failed to save screenshot to {}", output.display()))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn missing_image_preserves_output() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let output = dir.path().join("screen.png");

        fs::write(&output, "previous image")?;
        assert!(save_image(&output, None).is_err());
        assert_eq!(fs::read_to_string(output)?, "previous image");
        Ok(())
    }
}

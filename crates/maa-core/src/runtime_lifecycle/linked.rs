use std::path::Path;

use crate::{Assistant, Result};

impl Assistant {
    /// Do nothing, as MaaCore is dynamically linked
    pub fn load(_: impl AsRef<Path>) -> Result<()> {
        Ok(())
    }

    /// Do nothing, as MaaCore is dynamically linked
    pub fn unload() -> Result<()> {
        Ok(())
    }

    /// Always returns true, as MaaCore is dynamically linked.
    pub fn loaded() -> bool {
        true
    }
}

pub(crate) struct Guard;

impl Guard {
    pub(crate) fn acquire() -> Result<Self> {
        Ok(Self)
    }
}

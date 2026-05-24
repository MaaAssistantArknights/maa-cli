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

pub(crate) struct PendingAssistantRegistration;

impl PendingAssistantRegistration {
    pub(crate) fn begin() -> Result<Self> {
        Ok(Self)
    }

    pub(crate) fn commit(self) {}
}

pub(crate) fn unregister_assistant() {}

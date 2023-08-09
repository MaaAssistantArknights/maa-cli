use maa_utils::config::FromFile;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Update {
    #[serde(default = "default_mirror")]
    pub mirror: String,
}

pub fn default_mirror() -> String {
    String::from("https://github.com/MaaAssistantArknights/MaaAssistantArknights")
}

impl FromFile for Update {}

#[cfg(test)]
mod tests {}

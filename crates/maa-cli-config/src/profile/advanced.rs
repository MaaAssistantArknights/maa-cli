use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AdvancedConfig {
    pub inference_engine: Option<String>,
    pub user_resource: Option<bool>,
}

impl AdvancedConfig {
    pub fn merge(self, other: Self) -> Self {
        Self {
            inference_engine: other.inference_engine.or(self.inference_engine),
            user_resource: other.user_resource.or(self.user_resource),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advanced_partial_override() {
        let parent = AdvancedConfig {
            inference_engine: Some("cpu".into()),
            user_resource: Some(false),
        };
        let child = AdvancedConfig {
            inference_engine: None,
            user_resource: Some(true),
        };

        assert_eq!(parent.merge(child), AdvancedConfig {
            inference_engine: Some("cpu".into()),
            user_resource: Some(true),
        });
    }

    #[test]
    fn advanced_full_override() {
        let parent = AdvancedConfig {
            inference_engine: Some("cpu".into()),
            user_resource: Some(false),
        };
        let child = AdvancedConfig {
            inference_engine: Some("gpu:0".into()),
            user_resource: Some(true),
        };

        assert_eq!(parent.merge(child), AdvancedConfig {
            inference_engine: Some("gpu:0".into()),
            user_resource: Some(true),
        });
    }

    #[test]
    fn advanced_empty_child() {
        let parent = AdvancedConfig {
            inference_engine: Some("cpu".into()),
            user_resource: Some(false),
        };

        assert_eq!(parent.clone().merge(AdvancedConfig::default()), parent);
    }
}

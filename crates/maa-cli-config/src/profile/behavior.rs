use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BehaviorConfig {
    pub auto_reconnect: Option<bool>,
    pub deployment_with_pause: Option<bool>,
}

impl BehaviorConfig {
    pub fn merge(self, other: Self) -> Self {
        Self {
            auto_reconnect: other.auto_reconnect.or(self.auto_reconnect),
            deployment_with_pause: other.deployment_with_pause.or(self.deployment_with_pause),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavior_partial_override() {
        let parent = BehaviorConfig {
            auto_reconnect: Some(true),
            deployment_with_pause: Some(false),
        };
        let child = BehaviorConfig {
            auto_reconnect: None,
            deployment_with_pause: Some(true),
        };

        assert_eq!(parent.merge(child), BehaviorConfig {
            auto_reconnect: Some(true),
            deployment_with_pause: Some(true),
        });
    }

    #[test]
    fn behavior_full_override() {
        let parent = BehaviorConfig {
            auto_reconnect: Some(true),
            deployment_with_pause: Some(false),
        };
        let child = BehaviorConfig {
            auto_reconnect: Some(false),
            deployment_with_pause: Some(true),
        };

        assert_eq!(parent.merge(child), BehaviorConfig {
            auto_reconnect: Some(false),
            deployment_with_pause: Some(true),
        });
    }

    #[test]
    fn behavior_empty_child() {
        let parent = BehaviorConfig {
            auto_reconnect: Some(true),
            deployment_with_pause: Some(false),
        };

        assert_eq!(parent.clone().merge(BehaviorConfig::default()), parent);
    }
}

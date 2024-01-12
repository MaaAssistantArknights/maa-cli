use crate::{
    config::task::{Task, TaskConfig, TaskType},
    object,
    value::{
        userinput::{BoolInput, Input, SelectD, ValueWithDesc},
        MAAValue, Map,
    },
};

use anyhow::Result;
use clap::ValueEnum;

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Clone, Copy)]
pub enum Theme {
    Phantom,
    Mizuki,
    Sami,
}

impl Theme {
    fn to_str(self) -> &'static str {
        match self {
            Self::Phantom => "Phantom",
            Self::Mizuki => "Mizuki",
            Self::Sami => "Sami",
        }
    }
}

impl ValueEnum for Theme {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Phantom, Self::Mizuki, Self::Sami]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.to_str()))
    }
}

pub fn roguelike(theme: Theme) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    use MAAValue::OptionalInput;
    let params = object!(
        "theme" => theme.to_str(),
        "mode" => SelectD::<i64>::new([
            ValueWithDesc::new(0, Some("Clear as many stages as possible with stable strategy")),
            ValueWithDesc::new(1, Some("Invest ingots and exits after first level")),
            ValueWithDesc::new(3, Some("Clear as many stages as possible with agrressive strategy")),
            ValueWithDesc::new(4, Some("Exit after entering 3rd level")),
        ], Some(1), Some("Roguelike mode"), false).unwrap(),
        "start_count" => Input::<i64>::new(Some(999), Some("number of times to start a new run")),
        "investment_disabled" => BoolInput::new(Some(false), Some("disable investment")),
        "investments_count" => OptionalInput {
            deps: Map::from([("investment_disabled".to_string(), false.into())]),
            input: Input::<i64>::new(Some(999), Some("number of times to invest")).into(),
        },
        "stop_when_investment_full" => OptionalInput {
            deps: Map::from([("investment_disabled".to_string(), false.into())]),
            input: BoolInput::new(Some(false), Some("stop when investment is full")).into(),
        },
        "squad" => Input::<String>::new(None, Some("squad name")),
        "roles" => Input::<String>::new(None, Some("roles")),
        "core_char" => SelectD::<String>::new(
            ["百炼嘉维尔", "焰影苇草", "锏"],
            None,
            Some("core operator"),
            true,
        ).unwrap(),
        "use_support" => BoolInput::new(Some(false), Some("use support operator")),
        "use_nonfriend_support" => OptionalInput {
            deps: Map::from([("use_support".to_string(), true.into())]),
            input: BoolInput::new(Some(false), Some("use non-friend support operator")).into(),
        },
        "refresh_trader_with_dice" => OptionalInput {
            deps: Map::from([("theme".to_string(), "Mizuki".into())]),
            input: BoolInput::new(Some(false), Some("refresh trader with dice")).into(),
        },
    );

    task_config.push(Task::new_with_default(TaskType::Roguelike, params));

    Ok(task_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_to_str() {
        assert_eq!(Theme::Phantom.to_str(), "Phantom");
        assert_eq!(Theme::Mizuki.to_str(), "Mizuki");
        assert_eq!(Theme::Sami.to_str(), "Sami");
    }

    #[test]
    fn theme_value_variants() {
        assert_eq!(
            Theme::value_variants(),
            &[Theme::Phantom, Theme::Mizuki, Theme::Sami]
        );
    }

    #[test]
    fn theme_to_possible_value() {
        assert_eq!(
            Theme::Phantom.to_possible_value(),
            Some(clap::builder::PossibleValue::new("Phantom"))
        );
        assert_eq!(
            Theme::Mizuki.to_possible_value(),
            Some(clap::builder::PossibleValue::new("Mizuki"))
        );
        assert_eq!(
            Theme::Sami.to_possible_value(),
            Some(clap::builder::PossibleValue::new("Sami"))
        );
    }

    #[test]
    fn roguelike_task_config() {
        let task_config = roguelike(Theme::Phantom).unwrap();
        let tasks = task_config.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_type(), &TaskType::Roguelike);
        assert_eq!(
            tasks[0].params().get("theme").unwrap(),
            &MAAValue::from("Phantom")
        );
    }
}

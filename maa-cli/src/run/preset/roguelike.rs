use crate::{
    config::task::{Task, TaskConfig},
    object,
    value::userinput::{BoolInput, Input, SelectD, ValueWithDesc},
};

use anyhow::Result;
use clap::ValueEnum;
use maa_sys::TaskType::Roguelike;

#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Clone, Copy)]
pub enum Theme {
    Phantom,
    Mizuki,
    Sami,
    Sarkaz,
}

impl Theme {
    fn to_str(self) -> &'static str {
        match self {
            Self::Phantom => "Phantom",
            Self::Mizuki => "Mizuki",
            Self::Sami => "Sami",
            Self::Sarkaz => "Sarkaz",
        }
    }
}

impl ValueEnum for Theme {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Phantom, Self::Mizuki, Self::Sami, Self::Sarkaz]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.to_str()))
    }
}

pub fn roguelike(theme: Theme) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    let params = object!(
        "theme" => theme.to_str(),
        "mode" => SelectD::<i32>::new([
            ValueWithDesc::new(0, Some("Clear as many stages as possible with stable strategy")),
            ValueWithDesc::new(1, Some("Invest ingots and exits after first level")),
            ValueWithDesc::new(3, Some("Clear as many stages as possible with agrressive strategy")),
            ValueWithDesc::new(4, Some("Exit after entering 3rd level")),
        ], Some(1), Some("Roguelike mode"), false).unwrap(),
        "start_count" => Input::<i32>::new(Some(999), Some("number of times to start a new run")),
        "investment_disabled" => BoolInput::new(Some(false), Some("disable investment")),
        "investments_count" if "investment_disabled" == false =>
            Input::<i32>::new(Some(999), Some("number of times to invest")),
        "stop_when_investment_full" if "investment_disabled" == false =>
            BoolInput::new(Some(false), Some("stop when investment is full")),
        "squad" => Input::<String>::new(None, Some("squad name")),
        "roles" => Input::<String>::new(None, Some("roles")),
        "core_char" => SelectD::<String>::new(
            ["百炼嘉维尔", "焰影苇草", "锏", "维什戴尔"],
            None,
            Some("core operator"),
            true,
        ).unwrap(),
        "use_support" => BoolInput::new(Some(false), Some("use support operator")),
        "use_nonfriend_support" if "use_support" == true =>
            BoolInput::new(Some(false), Some("use non-friend support operator")),
        "refresh_trader_with_dice" if "theme" == "Mizuki" =>
            BoolInput::new(Some(false), Some("refresh trader with dice")),
    );

    task_config.push(Task::new_with_default(Roguelike, params));

    Ok(task_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::value::MAAValue;

    #[test]
    fn theme_to_str() {
        assert_eq!(Theme::Phantom.to_str(), "Phantom");
        assert_eq!(Theme::Mizuki.to_str(), "Mizuki");
        assert_eq!(Theme::Sami.to_str(), "Sami");
        assert_eq!(Theme::Sarkaz.to_str(), "Sarkaz");
    }

    #[test]
    fn theme_value_variants() {
        assert_eq!(
            Theme::value_variants(),
            &[Theme::Phantom, Theme::Mizuki, Theme::Sami, Theme::Sarkaz]
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
        assert_eq!(
            Theme::Sarkaz.to_possible_value(),
            Some(clap::builder::PossibleValue::new("Sarkaz"))
        );
    }

    #[test]
    fn roguelike_task_config() {
        let task_config = roguelike(Theme::Phantom).unwrap();
        let tasks = task_config.tasks();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_type(), Roguelike);
        assert_eq!(
            tasks[0].params().get("theme").unwrap(),
            &MAAValue::from("Phantom")
        );
    }
}

use crate::{
    config::task::{task_type::MAATask, Task, TaskConfig},
    object,
    value::MAAValue,
};

use anyhow::Result;
use clap::{Args, ValueEnum};

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

#[derive(Args)]
pub struct RoguelikeArgs {
    /// Roguelike theme
    ///
    /// - Phantom
    /// - Mizuki
    /// - Sami
    #[clap(default_value = "Sami")]
    theme: Theme,
    /// Roguelike mode, determine the strategy to use
    ///
    /// - 0: Clear as many stages as possible with stable strategy
    /// - 1: Invest ingots and exits after first level
    /// - 2: A combination of 0 and 1, depracated
    /// - 3: Clear as many stages as possible with agrressive strategy
    /// - 4: Exit after entering 3rd level
    #[clap(long, default_value_t = 0)]
    mode: i64,
    /// The number of times to start a new run
    #[clap(long)]
    start_count: Option<i64>,
    /// Disable investment
    #[clap(long)]
    investment_disabled: bool,
    /// The number of times to invest
    #[clap(long)]
    investments_count: Option<i64>,
    /// Stop when investment is full
    #[clap(long)]
    stop_when_investment_full: bool,
    /// Squad name
    #[clap(long)]
    squad: String,
    /// Roles
    #[clap(long)]
    roles: String,
    /// Core operator name
    #[clap(long)]
    core_char: String,
    /// Use support operator
    #[clap(long)]
    use_support: bool,
    /// Use non-friend support operator
    #[clap(long)]
    use_nonfriend_support: bool,
    /// Refresh trader with dice
    ///
    /// Only support in Mizuki theme
    #[clap(long)]
    refresh_trader_with_dice: bool,
}

impl RoguelikeArgs {
    fn into_params(self) -> MAAValue {
        let mut params = object!(
            "theme" => self.theme.to_str(),
            "mode" => self.mode,
            "investment_disabled" => self.investment_disabled,
            "stop_when_investment_full" => self.stop_when_investment_full,
            "squad" => self.squad,
            "roles" => self.roles,
            "core_char" => self.core_char,
            "use_support" => self.use_support,
            "use_nonfriend_support" => self.use_nonfriend_support,
            "refresh_trader_with_dice" => self.refresh_trader_with_dice,
        );

        if let Some(start_count) = self.start_count {
            params.insert("start_count", start_count);
        }

        if let Some(investments_count) = self.investments_count {
            params.insert("investments_count", investments_count);
        }

        params
    }
}

pub fn roguelike(args: RoguelikeArgs) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    task_config.push(Task::new_with_default(
        MAATask::Roguelike,
        args.into_params(),
    ));

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
}

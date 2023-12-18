use crate::{
    config::task::{task_type::MAATask, MAAValue, Task, TaskConfig},
    input::{BoolInput, Input, Select, SelectD, Selectable, UserInput, ValueWithDesc},
    object,
};

use std::convert::Infallible;

use anyhow::Result;
use clap::{Args, ValueEnum};

#[derive(ValueEnum, Clone, Copy)]
pub enum Theme {
    Phantom,
    Mizuki,
    Sami,
}

impl Theme {
    pub fn to_str(self) -> &'static str {
        match self {
            Theme::Phantom => "Phantom",
            Theme::Mizuki => "Mizuki",
            Theme::Sami => "Sami",
        }
    }
}

impl Selectable for Theme {
    type Value = String;
    type Error = Infallible;

    fn value(self) -> Self::Value {
        self.to_str().to_owned()
    }

    // TODO: localized description
    fn desc(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }

    fn parse(input: &str) -> std::prelude::v1::Result<Self::Value, Self::Error> {
        Ok(input.to_owned())
    }
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

#[derive(Args)]

/// Task parameters for task roguelike
///
/// ```json
/// {
///     "enable": bool,         // 是否启用本任务，可选，默认为 true
///     "theme": string,        // 肉鸽名，可选，默认 "Phantom"
///                             // Phantom - 傀影与猩红血钻
///                             // Mizuki  - 水月与深蓝之树
///                             // Sami    - 探索者的银凇止境
///     "mode": int,            // 模式，可选项。默认 0
///                             // 0 - 刷蜡烛，尽可能稳定地打更多层数
///                             // 1 - 刷源石锭，第一层投资完就退出
///                             // 2 - 【即将弃用】两者兼顾，投资过后再退出，没有投资就继续往后打
///                             // 3 - 开发中...
///                             // 4 - 烧热水，到达第三层后直接退出
///     "starts_count": int,    // 开始探索 次数，可选，默认 INT_MAX。达到后自动停止任务
///     "investment_enabled": bool, // 是否投资源石锭，默认开
///     "investments_count": int,
///                             // 投资源石锭 次数，可选，默认 INT_MAX。达到后自动停止任务
///     "stop_when_investment_full": bool,
///                             // 投资满了自动停止任务，可选，默认 false
///     "squad": string,        // 开局分队，可选，例如 "突击战术分队" 等，默认 "指挥分队"
///     "roles": string,        // 开局职业组，可选，例如 "先手必胜" 等，默认 "取长补短"
///     "core_char": string,    // 开局干员名，可选，仅支持单个干员中！文！名！。默认识别练度自动选择
///     "use_support": bool,  // 开局干员是否为助战干员，可选，默认 false
///     "use_nonfriend_support": bool,  // 是否可以是非好友助战干员，可选，默认 false，use_support为true时有效
///     "refresh_trader_with_dice": bool  // 是否用骰子刷新商店购买特殊商品，目前支持水月肉鸽的指路鳞，可选，默认 false
/// }
/// ```
pub struct RoguelikeArgs {
    /// Query arguments for task roguelike interactively, instead of using command line arguments
    ///
    /// If this is set to true, all other arguments will be ignored
    #[clap(short, long)]
    interactive: bool,
    /// Roguelike theme
    #[clap(default_value_t = Theme::Sami)]
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
    #[clap(long, default_value_t = 999)]
    start_count: i64,
    /// Disable investment
    #[clap(long, default_value_t = false)]
    investment_disabled: bool,
    /// The number of times to invest
    #[clap(long, default_value_t = 999)]
    investments_count: i64,
    /// Stop when investment is full
    #[clap(long, default_value_t = false)]
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
    #[clap(long, default_value_t = false)]
    use_support: bool,
    /// Use non-friend support operator
    #[clap(long, default_value_t = false)]
    use_nonfriend_support: bool,
    /// Refresh trader with dice
    ///
    /// Only support in Mizuki theme
    #[clap(long, default_value_t = false)]
    refresh_trader_with_dice: bool,
}

impl RoguelikeArgs {
    fn to_params(&self) -> Result<MAAValue> {
        if self.interactive {
            interactive_params()
        } else {
            let mut params = MAAValue::new();

            params.insert("theme", self.theme.to_str());

            Ok(params)
        }
    }
}

pub fn roguelike(args: RoguelikeArgs) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    let params = args.to_params()?;

    task_config.push(Task::new_with_default(MAATask::Roguelike, params));

    Ok(task_config)
}

fn interactive_params() -> Result<MAAValue> {
    let theme = Select::<Theme>::new(
        [Theme::Phantom, Theme::Mizuki, Theme::Sami],
        Some(3),
        Some("a roguelike theme"),
        true,
    )
    .value()?;

    let mut params = object!(
        "mode" => SelectD::<i64>::new(
            [
                ValueWithDesc::new(
                    0,
                    Some("Clear as many stages as possible with stable strategy"),
                ),
                ValueWithDesc::new(1, Some("Invest ingots and exits after first level")),
                ValueWithDesc::new(2, Some("A combination of 0 and 1, depracated")),
                ValueWithDesc::new(
                    3,
                    Some("Clear as many stages as possible with agrressive strategy"),
                ),
                ValueWithDesc::new(4, Some("Exit entering 3rd level")),
            ],
            Some(0),
            Some("a roguelike mode"),
            true,
        ),
        "start_count" => Input::<i64>::new(Some(9999), Some("the number of times to start a new run")),
        "investment_enabled" => BoolInput::new(Some(true), Some("enable investment")),
        "investments_count" => Input::<i64>::new(Some(99999), Some("the number of times to invest")),
        "stop_when_investment_full" => BoolInput::new(Some(false), Some("stop when investment is full")),
        "squad" => Input::<String>::new(None::<String>, Some("a squad name")),
        "roles" => Input::<String>::new(None::<String>, Some("roles")),
        "core_char" => Input::<String>::new(None::<String>, Some("a operator name")),
        "use_support" => BoolInput::new(Some(true), Some("use support operator")),
    );

    params.init()?;

    if params.get_or("use_support", false)? {
        params.insert(
            "use_nonfriend_support",
            BoolInput::new(Some(false), Some("use non-friend support operator")).value()?,
        );
    }

    if &theme == Theme::Mizuki.to_str() {
        params.insert(
            "refresh_trader_with_dice",
            BoolInput::new(Some(false), Some("refresh trader with dice")).value()?,
        );
    }

    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{assert_matches, config::task::task_type::TaskOrUnknown};

    #[test]
    fn theme_to_str() {
        assert_eq!(Theme::Phantom.to_str(), "Phantom");
        assert_eq!(Theme::Mizuki.to_str(), "Mizuki");
        assert_eq!(Theme::Sami.to_str(), "Sami");
    }
}

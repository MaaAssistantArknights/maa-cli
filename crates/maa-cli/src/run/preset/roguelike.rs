use anyhow::bail;
use clap::ValueEnum;

use super::MAAValue;

#[repr(i8)]
#[cfg_attr(test, derive(PartialEq, Debug))]
#[derive(Clone, Copy)]
pub enum Theme {
    Phantom,
    Mizuki,
    Sami,
    Sarkaz,
    JieGarden,
}

impl Theme {
    const fn to_str(self) -> &'static str {
        match self {
            Self::Phantom => "Phantom",
            Self::Mizuki => "Mizuki",
            Self::Sami => "Sami",
            Self::Sarkaz => "Sarkaz",
            Self::JieGarden => "JieGarden",
        }
    }
}

impl ValueEnum for Theme {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            Self::Phantom,
            Self::Mizuki,
            Self::Sami,
            Self::Sarkaz,
            Self::JieGarden,
        ]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.to_str()))
    }
}

#[derive(clap::Args)]
pub struct RoguelikeParams {
    /// Theme of the roguelike
    theme: Theme,
    /// Mode of the roguelike
    ///
    /// 0: mode for score;
    /// 1: mode for ingots;
    /// 2: combination of 0 and 1, deprecated;
    /// 3: mode for pass, not implemented yet;
    /// 4: mode that exist after 3rd floor;
    /// 5: mode for collapsal paradigms, only for Sami, use with `expected_collapsal_paradigms`
    #[arg(long, default_value = "0")]
    mode: i32,

    // TODO: input localized names, maybe during initialization of tasks

    // Start related parameters
    /// Squad to start with in Chinese, e.g. "指挥分队" (default), "后勤分队"
    #[arg(long)]
    squad: Option<String>,
    /// Starting core operator in Chinese, e.g. "维什戴尔"
    #[arg(long)]
    core_char: Option<String>,
    /// Starting operators recruitment combination in Chinese, e.g. "取长补短", "先手必胜"
    /// (default)
    #[arg(long)]
    roles: Option<String>,

    /// Stop after given count, if not given, never stop
    #[arg(long)]
    start_count: Option<i32>,

    /// Difficulty, not valid for Phantom theme (no numerical difficulty)
    ///
    /// If the given difficulty is larger than the maximum difficulty of the theme, it will be
    /// capped to the maximum difficulty. If not given, 0 will be used.
    #[arg(long)]
    difficulty: Option<i32>,

    // Investment related parameters
    /// Disable investment
    #[arg(long)]
    disable_investment: bool,
    /// Try to gain more score in investment mode
    ///
    /// By default, some actions will be skipped in investment mode to save time.
    /// If this option is enabled, try to gain exp score in investment mode.
    #[arg(long)]
    investment_with_more_score: bool,
    /// Stop exploration investment reaches given count
    #[arg(long)]
    investments_count: Option<i32>,
    /// Do not stop exploration when investment is full
    #[arg(long)]
    no_stop_when_investment_full: bool,

    // Support related parameters
    /// Use support operator
    #[arg(long)]
    use_support: bool,
    /// Use non-friend support operator
    #[arg(long)]
    use_nonfriend_support: bool,

    // Elite related parameters
    /// Start with elite two
    #[arg(long)]
    start_with_elite_two: bool,
    /// Only start with elite two
    #[arg(long)]
    only_start_with_elite_two: bool,

    /// Stop exploration before final boss
    #[arg(long)]
    stop_at_final_boss: bool,

    // Mizuki specific parameters
    /// Whether to refresh trader with dice (only available in Mizuki theme)
    #[arg(long)]
    refresh_trader_with_dice: bool,

    // Sami specific parameters
    // Foldartal related parameters
    /// Whether to use Foldartal in Sami theme
    #[arg(long)]
    use_foldartal: bool,
    /// A list of expected Foldartal to be started with
    #[arg(short = 'F', long)]
    start_foldartals: Vec<String>,
    /// A list of expected collapsal paradigms
    #[arg(short = 'P', long)]
    expected_collapsal_paradigms: Vec<String>,

    // Sarkaz specific parameters
    /// Whether to start with seed, only available in Sarkaz theme and mode 1
    #[arg(long)]
    start_with_seed: bool,
}

impl super::ToTaskType for RoguelikeParams {
    fn to_task_type(&self) -> super::TaskType {
        super::TaskType::Roguelike
    }
}

impl super::IntoParameters for RoguelikeParams {
    fn into_parameters_no_context(self) -> anyhow::Result<MAAValue> {
        let mut value = MAAValue::default();

        let theme = self.theme;
        let mode = self.mode;

        match mode {
            5 if !matches!(theme, Theme::Sami) => {
                bail!("Mode 5 is only available in Sami theme");
            }
            0..=5 => {}
            _ => bail!("Mode must be in range between 0 and 5"),
        }

        value.insert("theme", self.theme.to_str());
        value.insert("mode", self.mode);

        value.maybe_insert("squad", self.squad);
        value.maybe_insert("roles", self.roles);
        value.maybe_insert("core_char", self.core_char);

        value.maybe_insert("start_count", self.start_count);

        if matches!(theme, Theme::Phantom) {
            if self.difficulty.is_some() {
                log::warn!("Difficulty is not valid for Phantom theme, ignored");
            }
        } else {
            value.maybe_insert("difficulty", self.difficulty);
        }

        if self.disable_investment {
            value.insert("investment_enabled", false);
        } else {
            value.insert("investment_enabled", true);
            value.maybe_insert("investments_count", self.investments_count);
            value.insert(
                "investment_with_more_score",
                self.investment_with_more_score,
            );
            value.insert(
                "stop_when_investment_full",
                !self.no_stop_when_investment_full,
            );
        }

        if self.use_support {
            value.insert("use_support", true);
            value.insert("use_nonfriend_support", self.use_nonfriend_support);
        }

        if self.start_with_elite_two {
            value.insert("start_with_elite_two", true);
            value.insert("only_start_with_elite_two", self.only_start_with_elite_two);
        }

        value.insert("stop_at_final_boss", self.stop_at_final_boss);

        // Theme specific parameters
        match theme {
            Theme::Mizuki => {
                value.insert("refresh_trader_with_dice", self.refresh_trader_with_dice);
            }
            Theme::Sami => {
                value.insert("use_foldartal", self.use_foldartal);
                if !self.start_foldartals.is_empty() {
                    value.insert(
                        "start_foldartal_list",
                        MAAValue::Array(
                            self.start_foldartals
                                .into_iter()
                                .map(MAAValue::from)
                                .collect(),
                        ),
                    );
                }

                if mode == 5 {
                    value.insert("check_collapsal_paradigms", true);
                    value.insert("double_check_collapsal_paradigms", true);
                    if self.expected_collapsal_paradigms.is_empty() {
                        bail!(
                            "At least one expected collapsal paradigm is required when mode 5 is enabled"
                        );
                    }
                    value.insert(
                        "expected_collapsal_paradigms",
                        MAAValue::Array(
                            self.expected_collapsal_paradigms
                                .into_iter()
                                .map(MAAValue::from)
                                .collect(),
                        ),
                    );
                }
            }
            Theme::Sarkaz if mode == 1 => {
                value.insert("start_with_seed", self.start_with_seed);
            }
            _ => {}
        }

        Ok(value)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use maa_value::object;

    use super::*;
    use crate::command::{Command, parse_from};

    mod theme {
        use super::*;

        #[test]
        fn to_str() {
            assert_eq!(Theme::Phantom.to_str(), "Phantom");
            assert_eq!(Theme::Mizuki.to_str(), "Mizuki");
            assert_eq!(Theme::Sami.to_str(), "Sami");
            assert_eq!(Theme::Sarkaz.to_str(), "Sarkaz");
            assert_eq!(Theme::JieGarden.to_str(), "JieGarden");
        }

        #[test]
        fn value_variants() {
            assert_eq!(Theme::value_variants(), &[
                Theme::Phantom,
                Theme::Mizuki,
                Theme::Sami,
                Theme::Sarkaz,
                Theme::JieGarden,
            ]);
        }

        #[test]
        fn to_possible_value() {
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
            assert_eq!(
                Theme::JieGarden.to_possible_value(),
                Some(clap::builder::PossibleValue::new("JieGarden"))
            );
        }
    }

    #[test]
    fn parse_roguellike_params() {
        fn parse<I, T>(args: I) -> Result<MAAValue, anyhow::Error>
        where
            I: IntoIterator<Item = T>,
            T: Into<std::ffi::OsString> + Clone,
        {
            let command = parse_from(args).command;
            match command {
                Command::Roguelike { params, .. } => {
                    use super::super::{IntoParameters, TaskType, ToTaskType};
                    assert_eq!(params.to_task_type(), TaskType::Roguelike);
                    params.into_parameters_no_context()
                }
                _ => panic!("Not a Roguelike command"),
            }
        }

        let default_params = object!(
            "mode" => 0,
            "investment_enabled" => true,
            "investment_with_more_score" => false,
            "stop_when_investment_full" => true,
            "stop_at_final_boss" => false,
        );

        assert_eq!(
            parse(["maa", "roguelike", "Phantom"]).unwrap(),
            default_params.join(object!("theme" => "Phantom")),
        );
        assert!(parse(["maa", "roguelike", "Phantom", "--mode", "5"]).is_err());
        assert!(parse(["maa", "roguelike", "Phantom", "--mode", "7"]).is_err());

        // Difficulty is ignored for Phantom theme
        assert_eq!(
            parse(["maa", "roguelike", "Phantom", "--difficulty", "15"]).unwrap(),
            default_params.join(object!("theme" => "Phantom")),
        );

        assert_eq!(
            parse([
                "maa",
                "roguelike",
                "Sarkaz",
                "--squad",
                "蓝图测绘分队",
                "--roles",
                "取长补短",
                "--core-char",
                "维什戴尔",
                "--start-count=100",
                "--difficulty=15",
            ])
            .unwrap(),
            default_params.join(object!(
                "theme" => "Sarkaz",
                "squad" => "蓝图测绘分队",
                "roles" => "取长补短",
                "core_char" => "维什戴尔",
                "start_count" => 100,
                "difficulty" => 15,
            )),
        );

        assert_eq!(
            parse(["maa", "roguelike", "Sarkaz", "--disable-investment"]).unwrap(),
            // Can't use default_params here because some fields are removed in this case
            object!(
                "theme" => "Sarkaz",
                "mode" => 0,
                "investment_enabled" => false,
                "stop_at_final_boss" => false,
            ),
        );
        assert_eq!(
            parse([
                "maa",
                "roguelike",
                "Sarkaz",
                "--investment-with-more-score",
                "--investments-count=100",
                "--no-stop-when-investment-full"
            ])
            .unwrap(),
            default_params.join(object!(
                "theme" => "Sarkaz",
                "investment_with_more_score" => true,
                "investments_count" => 100,
                "stop_when_investment_full" => false,
            )),
        );

        assert_eq!(
            parse([
                "maa",
                "roguelike",
                "Sarkaz",
                "--use-support",
                "--use-nonfriend-support",
                "--start-with-elite-two",
                "--only-start-with-elite-two",
                "--stop-at-final-boss",
            ])
            .unwrap(),
            default_params.join(object!(
                "theme" => "Sarkaz",
                "use_support" => true,
                "use_nonfriend_support" => true,
                "start_with_elite_two" => true,
                "only_start_with_elite_two" => true,
                "stop_at_final_boss" => true,
            )),
        );

        assert_eq!(
            parse(["maa", "roguelike", "Mizuki"]).unwrap(),
            default_params.join(object!(
                "theme" => "Mizuki",
                "refresh_trader_with_dice" => false,
            )),
        );

        assert_eq!(
            parse(["maa", "roguelike", "Mizuki", "--refresh-trader-with-dice"]).unwrap(),
            default_params.join(object!(
                "theme" => "Mizuki",
                "refresh_trader_with_dice" => true,
            )),
        );

        assert_eq!(
            parse([
                "maa",
                "roguelike",
                "Sami",
                "--use-foldartal",
                "-F英雄",
                "-F大地"
            ])
            .unwrap(),
            default_params.join(object!(
                "theme" => "Sami",
                "use_foldartal" => true,
                "start_foldartal_list" => MAAValue::Array(vec![
                    MAAValue::from("英雄"),
                    MAAValue::from("大地"),
                ]),
            )),
        );
        assert!(parse(["maa", "roguelike", "Sami", "--mode", "5"]).is_err());
        assert_eq!(
            parse([
                "maa",
                "roguelike",
                "Sami",
                "--mode=5",
                "-P目空一些",
                "-P图像损坏",
            ])
            .unwrap(),
            default_params.join(object!(
                "theme" => "Sami",
                "mode" => 5,
                "use_foldartal" => false,
                "check_collapsal_paradigms" => true,
                "double_check_collapsal_paradigms" => true,
                "expected_collapsal_paradigms" => MAAValue::Array(vec![
                    MAAValue::from("目空一些"),
                    MAAValue::from("图像损坏"),
                ]),
            )),
        );

        assert_eq!(
            parse([
                "maa",
                "roguelike",
                "Sarkaz",
                "--mode=1",
                "--start-with-seed",
            ])
            .unwrap(),
            default_params.join(object!(
                "theme" => "Sarkaz",
                "mode" => 1,
                "start_with_seed" => true,
            )),
        );
    }
}

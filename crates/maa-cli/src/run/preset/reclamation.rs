use color_print::cstr;

use super::MAAValue;

#[repr(u8)]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Copy)]
enum Theme {
    Tales,
}

impl Theme {
    const fn to_str(self) -> &'static str {
        match self {
            Theme::Tales => "Tales",
        }
    }
}

impl clap::ValueEnum for Theme {
    fn value_variants<'a>() -> &'a [Self] {
        &[Theme::Tales]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.to_str()))
    }
}

#[derive(clap::Args)]
pub struct ReclamationParams {
    /// Theme of the reclamation algorithm
    ///
    /// - Tales: Tales Within the Sand
    theme: Theme,
    #[arg(short = 'm', long, default_value = "1",
        help = "Reclamation Algorithm task mode, 0 or 1",
        long_help = cstr!(
        r#"Reclamation Algorithm task mode

        0: farm prosperity points by repeatedly entering and exiting stages.
            <bold>This task mode should only be used when no save exists.</bold>
            <red><bold>Using it with an active save may result in losing your progress.</bold></red>
        1: farm prosperity points by crafting tools.
            <bold>This task mode should only be used when you already have a save and can craft certain tools.</bold>
            <bold>It is recommend to start task from a new calculation day.</bold>
            <red><bold>Using it may result in losing your progress after last calculation day.</bold></red>
    "#))]
    mode: i32,
    /// Name of tool to craft in mode 1
    #[arg(short = 'C', long, default_value = "荧光棒")]
    tools_to_craft: Vec<String>,
    /// Method to interactive with the add button when increasing the crafting quantity
    ///
    /// 0: increase the number by clicking the button.
    /// 1: increase the number by holding the button.
    #[arg(long, default_value = "0", verbatim_doc_comment)]
    increase_mode: i32,
    /// Number of batches in each game run, with each batch containing 99 items
    #[arg(long, default_value = "16")]
    num_craft_batches: i32,
}

impl super::ToTaskType for ReclamationParams {
    fn to_task_type(&self) -> super::TaskType {
        super::TaskType::Reclamation
    }
}

impl From<ReclamationParams> for MAAValue {
    fn from(params: ReclamationParams) -> Self {
        let mut value = MAAValue::new();
        value.insert("theme", params.theme.to_str());
        value.insert("mode", params.mode);

        if params.mode == 1 {
            value.insert("tools_to_craft", params.tools_to_craft);
            value.insert("increase_mode", params.increase_mode);
            value.insert("num_craft_batches", params.num_craft_batches);
        }
        value
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    mod theme {
        use clap::ValueEnum;

        use super::*;

        #[test]
        fn to_str() {
            assert_eq!(Theme::Tales.to_str(), "Tales");
        }

        #[test]
        fn value_variants() {
            assert_eq!(Theme::value_variants(), &[Theme::Tales]);
        }

        #[test]
        fn to_possible_value() {
            assert_eq!(
                Theme::Tales.to_possible_value(),
                Some(clap::builder::PossibleValue::new("Tales"))
            );
        }
    }

    #[test]
    fn parse_reclamation_params() {
        fn parse<I, T>(args: I) -> MAAValue
        where
            I: IntoIterator<Item = T>,
            T: Into<std::ffi::OsString> + Clone,
        {
            let command = crate::command::parse_from(args).command;
            match command {
                crate::Command::Reclamation { params, .. } => {
                    use super::super::{TaskType, ToTaskType};
                    assert_eq!(params.to_task_type(), TaskType::Reclamation);
                    params.into()
                }
                _ => panic!("Not a Reclamation command"),
            }
        }

        use crate::object;

        let base_params = object!("theme" => "Tales", "mode" => 1);

        assert_eq!(
            parse(["maa", "reclamation", "Tales"]),
            base_params.join(object!(
                "tools_to_craft" => ["荧光棒"],
                "increase_mode" => 0,
                "num_craft_batches" => 16,
            )),
        );
        assert_eq!(
            parse(["maa", "reclamation", "Tales", "-m0"]),
            base_params.join(object!("mode" => 0)),
        );
        assert_eq!(
            parse([
                "maa",
                "reclamation",
                "Tales",
                "-m1",
                "-CFoo",
                "-CBar",
                "--increase-mode=1",
                "--num-craft-batches=32"
            ]),
            base_params.join(object!(
                "mode" => 1,
                "tools_to_craft" => ["Foo", "Bar"],
                "increase_mode" => 1,
                "num_craft_batches" => 32,
            )),
        );
    }
}

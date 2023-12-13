use crate::{
    config::task::{
        task_type::MAATask,
        value::input::{BoolInput, Input},
        Task, TaskConfig, Value,
    },
    object,
};

use anyhow::Result;
use clap::ValueEnum;

#[derive(ValueEnum, Clone, Copy)]
pub enum Theme {
    Phantom,
    Mizuki,
    Sami,
}

impl Theme {
    pub fn to_str(&self) -> &'static str {
        match self {
            Theme::Phantom => "Phantom",
            Theme::Mizuki => "Mizuki",
            Theme::Sami => "Sami",
        }
    }
}

pub fn roguelike(theme: Option<Theme>) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    let theme = if let Some(theme) = theme {
        Value::String(theme.to_str().into())
    } else {
        Value::InputString(Input::<String>::new(Some("Phantom"), Some("theme")).into())
    };

    // TODO: better prompt and options
    task_config.push(Task::new_with_default(
        MAATask::Roguelike,
        object!(
            "theme" => theme,
            "mode" => Input::<i64>::new(Some(0), Some("mode")),
            "squad" => Input::<String>::new::<String, &str>(None, Some("a squad name")),
            "core_char" => Input::<String>::new::<String, &str>(None, Some("a operator name")),
            "use_support" => BoolInput::new(Some(true), Some("use support")),
        ),
    ));

    Ok(task_config)
}


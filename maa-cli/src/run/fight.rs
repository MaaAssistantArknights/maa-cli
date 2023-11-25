use crate::{
    config::task::{
        default_variants,
        task_type::TaskType,
        value::input::{BoolInput, Input, Select},
        Strategy, Task, TaskConfig, Value,
    },
    dirs::Dirs,
    object,
};

use super::{run, CommonArgs, Result};

pub fn fight(dirs: &Dirs, startup: bool, closedown: bool, common: CommonArgs) -> Result<()> {
    let mut task_config = TaskConfig::new();

    if startup {
        task_config.push(Task::new(
            TaskType::StartUp,
            object!(
                "start_game_enabled" => BoolInput::new(Some(true), Some("start game")),
                "client_type" => Select::<String>::new(
                    // TODO: a select type that accepts a enum (maybe a trait)
                    vec!["Official", "Bilibili", "Txwy", "YoStarEN", "YoStarJP", "YoStarKR"],
                    Some("client type"),
                ),
            ),
            Strategy::default(),
            default_variants(),
        ));
    }

    task_config.push(Task::new(
        TaskType::Fight,
        object!(
            "stage" => Input::<String>::new(Some("1-7"), Some("a stage to fight")),
            "medicine" => Input::<i64>::new(Some(0), Some("medicine to use")),
        ),
        Strategy::default(),
        default_variants(),
    ));

    if closedown {
        task_config.push(Task::new(
            TaskType::CloseDown,
            Value::default(),
            Strategy::default(),
            default_variants(),
        ));
    }

    run(dirs, task_config, common)
}

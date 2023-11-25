use crate::{
    config::task::{
        task_type::TaskType,
        value::input::{BoolInput, Input, Select},
        Task, TaskConfig, Value,
    },
    dirs::Dirs,
    object,
};

use super::{run, CommonArgs, Result};

pub fn fight(dirs: &Dirs, startup: bool, closedown: bool, common: CommonArgs) -> Result<()> {
    let mut task_config = TaskConfig::new();

    if startup {
        task_config.push(Task::new_with_default(
            TaskType::StartUp,
            object!(
                "start_game_enabled" => BoolInput::new(Some(true), Some("start game")),
                "client_type" => Select::<String>::new(
                    // TODO: a select type that accepts a enum (maybe a trait)
                    vec!["Official", "Bilibili", "Txwy", "YoStarEN", "YoStarJP", "YoStarKR"],
                    Some("client type"),
                ),
            ),
        ));
    }

    task_config.push(Task::new_with_default(
        TaskType::Fight,
        object!(
            "stage" => Input::<String>::new(Some("1-7"), Some("a stage to fight")),
            "medicine" => Input::<i64>::new(Some(0), Some("medicine to use")),
        ),
    ));

    if closedown {
        task_config.push(Task::new_with_default(
            TaskType::CloseDown,
            Value::default(),
        ));
    }

    run(dirs, task_config, common)
}

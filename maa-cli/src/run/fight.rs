use crate::{
    config::task::{
        task_type::MAATask,
        value::input::{BoolInput, Input, Select},
        Task, TaskConfig, Value,
    },
    object,
};

use anyhow::Result;

pub fn fight<S>(stage: Option<S>, startup: bool, closedown: bool) -> Result<TaskConfig>
where
    S: Into<String>,
{
    let mut task_config = TaskConfig::new();

    if startup {
        task_config.push(Task::new_with_default(
            MAATask::StartUp,
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

    let stage = if let Some(stage) = stage {
        Value::String(stage.into())
    } else {
        Input::<String>::new(Some("1-7"), Some("a stage to fight")).into()
    };

    task_config.push(Task::new_with_default(
        MAATask::Fight,
        object!(
            "stage" => stage,
            "medicine" => Input::<i64>::new(Some(0), Some("medicine to use")),
        ),
    ));

    if closedown {
        task_config.push(Task::new_with_default(MAATask::CloseDown, Value::default()));
    }

    Ok(task_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        assert_matches,
        config::task::{task_type::TaskOrUnknown, ClientType, InitializedTaskConfig},
    };

    #[test]
    fn test_fight() {
        assert_matches!(
            fight::<&str>(None, true, true).unwrap().init().unwrap(),
            InitializedTaskConfig {
                client_type: Some(ClientType::Official),
                start_app: true,
                close_app: true,
                tasks
            } if tasks.len() == 3 && {
                let fight = tasks[1].clone();
                fight.0 == TaskOrUnknown::MAATask(MAATask::Fight)
                    && fight.1.get("stage").unwrap().as_string().unwrap() == "1-7"
                    && fight.1.get("medicine").unwrap().as_int().unwrap() == 0
            }
        );

        assert_matches!(
            fight(Some("CE-6"), false, false).unwrap().init().unwrap(),
            InitializedTaskConfig {
                client_type: None,
                start_app: false,
                close_app: false,
                tasks
            } if tasks.len() == 1 && {
                let fight = tasks[0].clone();
                fight.0 == TaskOrUnknown::MAATask(MAATask::Fight)
                    && fight.1.get("stage").unwrap().as_string().unwrap() == "CE-6"
                    && fight.1.get("medicine").unwrap().as_int().unwrap() == 0
            }
        )
    }
}

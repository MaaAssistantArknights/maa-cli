use crate::{
    config::task::{task_type::MAATask, ClientType, MAAValue, Task, TaskConfig},
    input::{BoolInput, Input, SelectD, ValueWithDesc},
    object,
};

use anyhow::Result;

impl From<ClientType> for ValueWithDesc<String> {
    fn from(client: ClientType) -> Self {
        Self::WithDesc {
            value: client.to_string(),
            // TODO: localized description
            desc: client.to_string(),
        }
    }
}

pub fn fight<S>(stage: Option<S>, startup: bool, closedown: bool) -> Result<TaskConfig>
where
    S: Into<String>,
{
    let mut task_config = TaskConfig::new();

    if startup {
        use ClientType::*;
        task_config.push(Task::new_with_default(
            MAATask::StartUp,
            object!(
                "start_game_enabled" => BoolInput::new(Some(true), Some("start game")),
                "client_type" => SelectD::<String>::new(
                    vec![Official, Bilibili, Txwy, YoStarEN, YoStarJP, YoStarKR],
                    Some(1),
                    Some("client type"),
                    true,
                ),
            ),
        ));
    }

    let stage = if let Some(stage) = stage {
        MAAValue::String(stage.into())
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
        task_config.push(Task::new_with_default(
            MAATask::CloseDown,
            MAAValue::default(),
        ));
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
                let fight = &tasks[1];
                fight.task_type() == &TaskOrUnknown::MAATask(MAATask::Fight)
                    && fight.params().get("stage").unwrap().as_string().unwrap() == "1-7"
                    && fight.params().get("medicine").unwrap().as_int().unwrap() == 0
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
                let fight = &tasks[0];
                fight.task_type() == &TaskOrUnknown::MAATask(MAATask::Fight)
                    && fight.params().get("stage").unwrap().as_string().unwrap() == "CE-6"
                    && fight.params().get("medicine").unwrap().as_int().unwrap() == 0
            }
        )
    }
}

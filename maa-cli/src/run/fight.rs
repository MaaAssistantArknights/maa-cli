use crate::{
    config::task::{task_type::MAATask, ClientType, Task, TaskConfig},
    object,
    value::{
        userinput::{BoolInput, Input, SelectD, ValueWithDesc},
        MAAValue,
    },
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

pub fn fight(stage: String, startup: bool, closedown: bool) -> Result<TaskConfig> {
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
                ).unwrap(),
            ),
        ));
    }

    task_config.push(Task::new_with_default(
        MAATask::Fight,
        object!(
            "stage" => stage,
            "medicine" => Input::new(Some(0), Some("medicine to use")),
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

    use crate::config::task::{InitializedTask, InitializedTaskConfig};

    #[test]
    fn test_fight() {
        use ClientType::*;

        assert_eq!(
            fight("1-7".to_string(), true, true)
                .unwrap()
                .init()
                .unwrap(),
            InitializedTaskConfig {
                client_type: Some(ClientType::Official),
                start_app: true,
                close_app: true,
                tasks: vec![
                    InitializedTask::new_noname(
                        MAATask::StartUp,
                        object!(
                            "start_game_enabled" => true,
                            "client_type" => Official.as_ref(),
                        ),
                    ),
                    InitializedTask::new_noname(
                        MAATask::Fight,
                        object!(
                            "stage" => "1-7",
                            "medicine" => 0,
                        ),
                    ),
                    InitializedTask::new_noname(MAATask::CloseDown, object!()),
                ],
            }
        );

        assert_eq!(
            fight("CE-6".to_string(), false, false)
                .unwrap()
                .init()
                .unwrap(),
            InitializedTaskConfig {
                client_type: None,
                start_app: false,
                close_app: false,
                tasks: vec![InitializedTask::new_noname(
                    MAATask::Fight,
                    object!(
                        "stage" => "CE-6",
                        "medicine" => 0,
                    ),
                )],
            }
        )
    }
}

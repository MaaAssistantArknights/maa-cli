use crate::{
    config::task::{ClientType, Task, TaskConfig},
    object,
    value::{
        userinput::{BoolInput, Input, SelectD, ValueWithDesc},
        MAAValue, Map,
    },
};

use anyhow::Result;
use maa_sys::TaskType::*;

impl From<ClientType> for ValueWithDesc<String> {
    fn from(client: ClientType) -> Self {
        Self::Value(client.to_string())
    }
}

pub fn fight(stage: String, startup: bool, closedown: bool) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    use MAAValue::OptionalInput;
    if startup {
        use ClientType::*;
        task_config.push(Task::new_with_default(
            StartUp,
            object!(
                "start_game_enabled" => BoolInput::new(Some(true), Some("start game")),
                "client_type" => OptionalInput {
                    deps: Map::from([("start_game_enabled".to_string(), true.into())]),
                    input: SelectD::<String>::new(
                        vec![Official, Bilibili, Txwy, YoStarEN, YoStarJP, YoStarKR],
                        Some(1),
                        Some("a client type"),
                        true,
                    ).unwrap().into(),
                }
            ),
        ));
    }

    task_config.push(Task::new_with_default(
        Fight,
        object!(
            "stage" => stage,
            "medicine" => Input::new(Some(0), Some("the number of medicine to use")),
        ),
    ));

    if closedown {
        task_config.push(Task::new_with_default(CloseDown, MAAValue::default()));
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
                        StartUp,
                        object!(
                            "start_game_enabled" => true,
                            "client_type" => Official.as_ref(),
                        ),
                    ),
                    InitializedTask::new_noname(
                        Fight,
                        object!(
                            "stage" => "1-7",
                            "medicine" => 0,
                        ),
                    ),
                    InitializedTask::new_noname(CloseDown, object!()),
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
                    Fight,
                    object!(
                        "stage" => "CE-6",
                        "medicine" => 0,
                    ),
                )],
            }
        )
    }
}

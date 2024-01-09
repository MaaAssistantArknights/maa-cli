use crate::{
    config::task::{task_type::TaskType, ClientType, Task, TaskConfig},
    object,
    value::{userinput::Input, MAAValue},
};

use anyhow::Result;

pub fn startup(client: Option<ClientType>) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    let params = if let Some(client) = client {
        object!(
            "client_type" => client.to_str(),
            "start_game_enabled" => true,
        )
    } else {
        object!()
    };

    task_config.push(Task::new_with_default(TaskType::StartUp, params));

    Ok(task_config)
}

pub fn closedown() -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    task_config.push(Task::new_with_default(TaskType::CloseDown, object!()));

    Ok(task_config)
}

pub fn fight(stage: String) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    task_config.push(Task::new_with_default(
        TaskType::Fight,
        object!(
            "stage" => stage,
            "medicine" => Input::new(Some(0), Some("the number of medicine to use")),
        ),
    ));

    Ok(task_config)
}

mod copilot;
pub use copilot::copilot;

mod roguelike;
pub use roguelike::{roguelike, Theme as RoguelikeTheme};

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::task::{InitializedTask, InitializedTaskConfig};

    #[test]
    fn test_fight() {
        assert_eq!(
            fight("CE-6".to_string()).unwrap().init().unwrap(),
            InitializedTaskConfig {
                client_type: None,
                start_app: false,
                close_app: false,
                tasks: vec![InitializedTask::new_noname(
                    TaskType::Fight,
                    object!(
                        "stage" => "CE-6",
                        "medicine" => 0,
                    ),
                )],
            }
        )
    }
}

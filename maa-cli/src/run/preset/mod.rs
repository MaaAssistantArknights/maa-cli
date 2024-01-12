use crate::{
    config::task::{task_type::MAATask, ClientType, Task, TaskConfig},
    object,
    value::{userinput::Input, MAAValue},
};

use anyhow::Result;

pub fn startup(client: Option<ClientType>, account: Option<String>) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    let mut params = MAAValue::new();

    if let Some(client) = client {
        params.insert("client_type", client.as_ref());
        params.insert("start_game_enabled", true);
    };

    if let Some(account) = account {
        params.insert("account_name", account);
    };

    task_config.push(Task::new_with_default(MAATask::StartUp, params));

    Ok(task_config)
}

pub fn closedown() -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    task_config.push(Task::new_with_default(MAATask::CloseDown, object!()));

    Ok(task_config)
}

pub fn fight(stage: String) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    task_config.push(Task::new_with_default(
        MAATask::Fight,
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

    #[test]
    fn test_startup() {
        let task_config = startup(None, None).unwrap();
        let tasks = task_config.tasks();

        assert_eq!(tasks.len(), 1);
        let startup_task = tasks.first().unwrap();

        assert_eq!(startup_task.task_type(), MAATask::StartUp);
    }
}

use crate::{
    config::task::{ClientType, Task, TaskConfig},
    object,
    value::MAAValue,
};

use anyhow::Result;
use maa_sys::TaskType::*;

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

    task_config.push(Task::new_with_default(StartUp, params));

    Ok(task_config)
}

pub fn closedown() -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    task_config.push(Task::new_with_default(CloseDown, object!()));

    Ok(task_config)
}

pub fn fight(stage: String, medicine: Option<i32>) -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    let mut params = MAAValue::new();

    params.insert("stage", stage);

    if let Some(medicine) = medicine {
        params.insert("medicine", medicine);
    };

    task_config.push(Task::new_with_default(Fight, params));

    Ok(task_config)
}

pub fn depot() -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    task_config.push(Task::new_with_default(Depot, object!()));

    Ok(task_config)
}

pub fn oper_box() -> Result<TaskConfig> {
    let mut task_config = TaskConfig::new();

    task_config.push(Task::new_with_default(OperBox, object!()));

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

        assert_eq!(startup_task.task_type(), StartUp);
        assert_eq!(startup_task.params().get("client_type"), None);
        assert_eq!(startup_task.params().get("start_game_enabled"), None);

        let task_config = startup(Some(ClientType::Official), None).unwrap();
        let tasks = task_config.tasks();
        let startup_task = tasks.first().unwrap();
        assert_eq!(
            startup_task
                .params()
                .get("client_type")
                .unwrap()
                .as_str()
                .unwrap(),
            "Official"
        );
        assert!(startup_task
            .params()
            .get("start_game_enabled")
            .unwrap()
            .as_bool()
            .unwrap());

        let task_config = startup(None, Some("test".to_owned())).unwrap();
        let tasks = task_config.tasks();
        let startup_task = tasks.first().unwrap();
        assert_eq!(
            startup_task
                .params()
                .get("account_name")
                .unwrap()
                .as_str()
                .unwrap(),
            "test"
        );
    }

    #[test]
    fn test_closedown() {
        let task_config = closedown().unwrap();
        let tasks = task_config.tasks();

        assert_eq!(tasks.len(), 1);
        let closedown_task = tasks.first().unwrap();

        assert_eq!(closedown_task.task_type(), CloseDown);
    }

    #[test]
    fn test_fight() {
        let task_config = fight("1-1".to_owned(), None).unwrap();
        let tasks = task_config.tasks();

        assert_eq!(tasks.len(), 1);
        let fight_task = tasks.first().unwrap();

        assert_eq!(fight_task.task_type(), Fight);
        assert_eq!(
            fight_task.params().get("stage").unwrap().as_str().unwrap(),
            "1-1"
        );
        assert_eq!(fight_task.params().get("medicine"), None);

        let task_config = fight("1-1".to_owned(), Some(1)).unwrap();
        let tasks = task_config.tasks();
        let fight_task = tasks.first().unwrap();
        assert_eq!(
            fight_task
                .params()
                .get("medicine")
                .unwrap()
                .as_int()
                .unwrap(),
            1
        );
    }

    #[test]
    fn test_depot() {
        let task_config = depot().unwrap();
        let tasks = task_config.tasks();

        assert_eq!(tasks.len(), 1);
        let depot_task = tasks.first().unwrap();

        assert_eq!(depot_task.task_type(), Depot);
    }

    #[test]
    fn test_oper_box() {
        let task_config = oper_box().unwrap();
        let tasks = task_config.tasks();

        assert_eq!(tasks.len(), 1);
        let oper_box_task = tasks.first().unwrap();

        assert_eq!(oper_box_task.task_type(), OperBox);
    }
}

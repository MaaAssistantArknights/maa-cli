use crate::{
    config::task::{
        default_variants, task_type::TaskType, value::input::Input, Strategy, Task, TaskList, Value,
    },
    dirs::Dirs,
};

use super::{run, Result};

pub fn fight(
    dirs: &Dirs,
    addr: Option<String>,
    user_resource: bool,
    batch: bool,
    startup: bool,
    closedown: bool,
) -> Result<()> {
    let mut task_list: Vec<Task> = Vec::new();
    if startup {
        task_list.push(Task::new(
            TaskType::StartUp,
            Value::default(),
            Strategy::default(),
            default_variants(),
        ));
    }

    let mut fight_params = Value::new();
    fight_params.insert(
        "stage",
        Value::InputString(Input::new(Some("1-7"), Some("a stage to fight")).into()),
    );
    fight_params.insert(
        "medicine",
        Value::InputInt(Input::new(Some(0), Some("medicine to use")).into()),
    );
    task_list.push(Task::new(
        TaskType::Fight,
        fight_params,
        Strategy::default(),
        default_variants(),
    ));

    if closedown {
        task_list.push(Task::new(
            TaskType::CloseDown,
            Value::default(),
            Strategy::default(),
            default_variants(),
        ));
    }

    let task = TaskList { tasks: task_list };

    run(dirs, task, addr, user_resource, batch, false)
}

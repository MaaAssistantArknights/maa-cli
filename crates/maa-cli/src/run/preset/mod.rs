use anyhow::{Context, Result};
use maa_sys::TaskType;
use maa_value::MAAValue;

use crate::config::{
    FindFileOrDefault,
    asst::AsstConfig,
    task::{ClientType, Task, TaskConfig},
};

fn default_file(task_type: TaskType) -> std::path::PathBuf {
    maa_dirs::join!(
        maa_dirs::config(),
        "overlays",
        task_type.to_str().to_lowercase()
    )
}

trait ToTaskType {
    fn to_task_type(&self) -> TaskType;
}

trait IntoParameters {
    fn into_parameters(self, config: &AsstConfig) -> Result<MAAValue>;
}

pub trait IntoTaskConfig {
    fn into_task_config(self, config: &AsstConfig) -> Result<TaskConfig>;
}

impl<T> IntoTaskConfig for T
where
    T: ToTaskType + IntoParameters,
{
    fn into_task_config(self, config: &AsstConfig) -> Result<TaskConfig> {
        let task_type = self.to_task_type();
        let params: MAAValue = self.into_parameters(config)?;

        let mut default = MAAValue::find_file_or_default(default_file(task_type))
            .context("Failed to load default task config")?;

        default.merge_from(&params);

        let mut task_config = TaskConfig::new();

        task_config.push(Task::new(task_type, default));

        Ok(task_config)
    }
}

#[derive(clap::Args)]
pub(crate) struct StartUpParams {
    client_type: Option<ClientType>,
    #[arg(long, alias = "account")]
    account_name: Option<String>,
}

impl ToTaskType for StartUpParams {
    fn to_task_type(&self) -> TaskType {
        TaskType::StartUp
    }
}

impl IntoParameters for StartUpParams {
    fn into_parameters(self, _: &AsstConfig) -> Result<MAAValue> {
        let mut value = MAAValue::default();

        if let Some(client_type) = self.client_type {
            value.insert("start_game_enabled", true);
            value.insert("client_type", client_type.to_str());
        }

        value.maybe_insert("account_name", self.account_name);

        Ok(value)
    }
}

#[derive(clap::Args)]
pub(crate) struct CloseDownParams {
    #[arg(default_value = "Official")]
    client: ClientType,
}

impl ToTaskType for CloseDownParams {
    fn to_task_type(&self) -> TaskType {
        TaskType::CloseDown
    }
}

impl IntoParameters for CloseDownParams {
    fn into_parameters(self, _: &AsstConfig) -> Result<MAAValue> {
        let mut value = MAAValue::default();
        value.insert("client_type", self.client.to_str());
        Ok(value)
    }
}

mod fight;
pub use fight::FightParams;

mod copilot;
pub use copilot::{CopilotParams, SSSCopilotParams};

mod roguelike;
pub use roguelike::RoguelikeParams;

mod reclamation;
pub use reclamation::ReclamationParams;

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use maa_dirs::Ensure;
    use maa_value::object;

    use super::*;
    use crate::command::{Command, parse_from};

    #[test]
    #[ignore = "write to user directory"]
    fn into_task_config() {
        struct TestParams {
            bar: Option<i32>,
        }

        impl ToTaskType for TestParams {
            fn to_task_type(&self) -> TaskType {
                TaskType::Custom
            }
        }

        impl IntoParameters for TestParams {
            fn into_parameters(self, _: &AsstConfig) -> Result<MAAValue> {
                let mut value = MAAValue::default();
                if let Some(bar) = self.bar {
                    value.insert("bar", bar);
                }
                Ok(value)
            }
        }

        let config = AsstConfig::default();
        let default = default_file(TaskType::Custom).with_extension("toml");

        // Ensure clean state - remove overlay file if it exists
        let _ = std::fs::remove_file(&default);

        // Test without overlay file and without CLI args
        let task_config = TestParams { bar: None }
            .into_task_config(&config)
            .unwrap()
            .init()
            .unwrap()
            .tasks;
        assert_eq!(task_config.len(), 1);
        assert_eq!(task_config[0].task_type, TaskType::Custom);
        assert_eq!(task_config[0].params, object!());

        // Create overlay file with foo = 42
        default.parent().unwrap().ensure().unwrap();
        let mut file = std::fs::File::create(&default).unwrap();
        use std::io::Write;
        writeln!(file, "foo = 42").unwrap();
        drop(file);

        // Test with overlay file but without CLI args - should use overlay values
        let task_config = TestParams { bar: None }
            .into_task_config(&config)
            .unwrap()
            .init()
            .unwrap()
            .tasks;
        assert_eq!(task_config.len(), 1);
        assert_eq!(task_config[0].task_type, TaskType::Custom);
        assert_eq!(task_config[0].params, object!("foo" => 42));

        // Test with overlay file and CLI args - CLI should override overlay
        let mut file = std::fs::File::create(&default).unwrap();
        writeln!(file, "foo = 42").unwrap();
        writeln!(file, "bar = 100").unwrap();
        drop(file);

        let task_config = TestParams { bar: Some(200) }
            .into_task_config(&config)
            .unwrap()
            .init()
            .unwrap()
            .tasks;
        assert_eq!(task_config.len(), 1);
        assert_eq!(task_config[0].task_type, TaskType::Custom);
        // CLI arg "bar = 200" should override overlay "bar = 100"
        // Overlay "foo = 42" should be preserved
        assert_eq!(task_config[0].params, object!("foo" => 42, "bar" => 200));

        // Clean up
        let _ = std::fs::remove_file(&default);
    }

    #[test]
    fn parse_startup_params() {
        fn parse<I, T>(args: I) -> MAAValue
        where
            I: IntoIterator<Item = T>,
            T: Into<std::ffi::OsString> + Clone,
        {
            let command = parse_from(args).command;
            match command {
                Command::StartUp { params, .. } => {
                    assert_eq!(params.to_task_type(), TaskType::StartUp);
                    params.into_parameters(&AsstConfig::default()).unwrap()
                }
                _ => panic!("Not a StartUp command"),
            }
        }

        assert_eq!(parse(["maa", "startup"]), object!());

        assert_eq!(
            parse(["maa", "startup", "Official"]),
            object!(
                "client_type" => "Official",
                "start_game_enabled" => true
            )
        );

        assert_eq!(
            parse(["maa", "startup", "YoStarEN", "--account", "account"]),
            object!(
                "client_type" => "YoStarEN",
                "start_game_enabled" => true,
                "account_name" => "account"
            )
        );
    }

    #[test]
    fn parse_closedown_params() {
        fn parse<I, T>(args: I) -> MAAValue
        where
            I: IntoIterator<Item = T>,
            T: Into<std::ffi::OsString> + Clone,
        {
            let cmd = parse_from(args).command;
            match cmd {
                Command::CloseDown { params, .. } => {
                    assert_eq!(params.to_task_type(), TaskType::CloseDown);
                    params.into_parameters(&AsstConfig::default()).unwrap()
                }
                _ => panic!("Not a CloseDown command"),
            }
        }

        assert_eq!(
            parse(["maa", "closedown"]),
            object!("client_type" => "Official")
        );

        assert_eq!(
            parse(["maa", "closedown", "YoStarEN"]),
            object!("client_type" => "YoStarEN")
        );
    }
}

use crate::{
    config::{
        asst::AsstConfig,
        task::{ClientType, Task, TaskConfig},
        FindFileOrDefault,
    },
    value::MAAValue,
};

use anyhow::{Context, Result};
use maa_sys::TaskType;

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

pub trait IntoTaskConfig {
    fn into_task_config(self, config: &AsstConfig) -> Result<TaskConfig>;
}

impl<T> IntoTaskConfig for T
where
    T: ToTaskType + TryInto<MAAValue>,
    T::Error: Into<anyhow::Error>,
{
    fn into_task_config(self, _: &AsstConfig) -> Result<TaskConfig> {
        let task_type = self.to_task_type();
        let mut params: MAAValue = self.try_into().map_err(Into::into)?;

        let default = MAAValue::find_file_or_default(default_file(task_type))
            .context("Failed to load default task config")?;

        params.merge_mut(&default);

        let mut task_config = TaskConfig::new();

        task_config.push(Task::new(task_type, params));

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

impl From<StartUpParams> for MAAValue {
    fn from(args: StartUpParams) -> Self {
        let mut value = MAAValue::new();

        if let Some(client_type) = args.client_type {
            value.insert("start_game_enabled", true);
            value.insert("client_type", client_type.to_str());
        }

        value.maybe_insert("account_name", args.account_name);

        value
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

impl From<CloseDownParams> for MAAValue {
    fn from(args: CloseDownParams) -> Self {
        let mut value = MAAValue::new();
        value.insert("client_type", args.client.to_str());
        value
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
mod tests {
    use super::*;

    use crate::{
        command::{parse_from, Command},
        object,
    };

    use maa_dirs::Ensure;

    impl MAAValue {
        /// Merge another value into this default value.
        ///
        /// Common use for test with default value.
        pub(super) fn join(&self, other: MAAValue) -> MAAValue {
            let mut value = self.clone();
            value.merge_mut(&other);
            value
        }
    }

    #[test]
    #[ignore = "write to user directory"]
    fn into_task_config() {
        struct TestParams;

        impl ToTaskType for TestParams {
            fn to_task_type(&self) -> TaskType {
                TaskType::Custom
            }
        }

        impl From<TestParams> for MAAValue {
            fn from(_: TestParams) -> Self {
                object!()
            }
        }

        let config = AsstConfig::default();

        let task_config = TestParams
            .into_task_config(&config)
            .unwrap()
            .init()
            .unwrap()
            .tasks;
        assert_eq!(task_config.len(), 1);
        assert_eq!(task_config[0].task_type, TaskType::Custom);
        assert_eq!(task_config[0].params, object!());

        let default = default_file(TaskType::Custom).with_extension("toml");
        default.parent().unwrap().ensure().unwrap();
        let mut file = std::fs::File::create(&default).unwrap();
        use std::io::Write;
        writeln!(file, "foo = 42").unwrap();
        let task_config = TestParams
            .into_task_config(&config)
            .unwrap()
            .init()
            .unwrap()
            .tasks;
        assert_eq!(task_config.len(), 1);
        assert_eq!(task_config[0].task_type, TaskType::Custom);
        assert_eq!(task_config[0].params, object!("foo" => 42));
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
                    params.into()
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
                    params.into()
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

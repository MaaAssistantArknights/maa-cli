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

pub trait IntoTaskConfig {
    fn into_task_config(self, config: &AsstConfig) -> Result<TaskConfig>;
}

trait ToTaskType {
    fn to_task_type(&self) -> TaskType;
}

impl<T> IntoTaskConfig for T
where
    T: ToTaskType + TryInto<MAAValue>,
    T::Error: Into<anyhow::Error>,
{
    fn into_task_config(self, _: &AsstConfig) -> Result<TaskConfig> {
        let task_type = self.to_task_type();
        let mut params: MAAValue = self.try_into().map_err(Into::into)?;

        let default = MAAValue::find_file_or_default(task_type.to_str().to_lowercase())
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
    fn into_task_config() {
        let config = AsstConfig::default();
        let params = StartUpParams {
            client_type: Some(ClientType::Official),
            account_name: Some("account".to_string()),
        };

        // let task_config = params.into_task_config(&config).unwrap();
        // let task = task_config.
        //
        // assert_eq!(task.task_type, TaskType::StartUp);
        // assert_eq!(
        //     task.params,
        //     default.join(object!("account_name" => "account"))
        // );
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

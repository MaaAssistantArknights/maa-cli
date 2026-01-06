use anyhow::{Context, bail};

use super::MAAValue;
use crate::config::task::ClientType;

#[derive(clap::Args)]
pub struct RecruitParams {
    /// Whether to refresh 3-star tags
    #[arg(long)]
    refresh: bool,

    /// Tag levels to select
    ///
    /// Specify multiple levels by repeating this option, e.g. `-s3 -s4` to select 3 and 4 star
    /// tags.
    #[arg(short, long = "select", action = clap::ArgAction::Append)]
    select: Vec<i32>,

    /// Tag levels to confirm
    ///
    /// Specify multiple levels by repeating this option, e.g. `-c3 -c4` to confirm 3 and 4 star
    /// tags. If you only want to calculate recruitment, set this to empty array.
    #[arg(short, long = "confirm", action = clap::ArgAction::Append)]
    confirm: Vec<i32>,

    /// Preferred tags for level 3 recruitment
    ///
    /// These tags will be forcefully selected when available for level 3 recruitment.
    #[arg(short = 'F', long = "first-tag", action = clap::ArgAction::Append)]
    first_tags: Vec<String>,

    /// Select more tags mode
    ///
    /// - 0: default behavior
    /// - 1: select 3 tags even if they may conflict
    /// - 2: if possible, select more high-star tag combinations even if they may conflict
    #[arg(long, default_value = "0")]
    extra_tags_mode: i32,

    /// Number of recruitment times
    ///
    /// If you only want to calculate recruitment, set this to 0.
    #[arg(short, long, default_value = "0")]
    times: i32,

    /// Whether to set recruitment time limit
    ///
    /// Only effective when times is 0.
    #[arg(long, default_value = "true")]
    set_time: bool,

    /// Whether to use expedited permits
    #[arg(long)]
    expedite: bool,

    /// Number of expedited permits to use
    ///
    /// Only effective when expedite is true.
    /// Leave empty for unlimited (until times limit is reached).
    #[arg(long)]
    expedite_times: Option<i32>,

    /// Whether to skip when robot tag is identified
    #[arg(long, default_value = "true")]
    skip_robot: bool,

    /// Recruitment time limit for each tag level (in minutes)
    ///
    /// Format: level=minutes, e.g. `--recruitment-time=3=540 --recruitment-time=4=540`.
    /// Default is 540 (09:00:00) for all levels.
    #[arg(long, action = clap::ArgAction::Append)]
    recruitment_time: Vec<String>,

    /// Whether to report to Penguin Statistics
    #[arg(long)]
    report_to_penguin: bool,

    /// Penguin Statistics ID for reporting
    ///
    /// Leave empty to report anonymously. Only effective when report_to_penguin is true.
    #[arg(long)]
    penguin_id: Option<String>,

    /// Whether to report to yituliu
    #[arg(long)]
    report_to_yituliu: bool,

    /// Yituliu ID for reporting
    ///
    /// Leave empty to report anonymously. Only effective when report_to_yituliu is true.
    #[arg(long)]
    yituliu_id: Option<String>,

    /// Server type, affects data reporting
    #[arg(long)]
    server: Option<ClientType>,
}

impl super::ToTaskType for RecruitParams {
    fn to_task_type(&self) -> super::TaskType {
        super::TaskType::Recruit
    }
}

impl super::IntoParameters for RecruitParams {
    fn into_parameters_no_context(self) -> anyhow::Result<MAAValue> {
        let mut params = MAAValue::default();

        params.insert("refresh", self.refresh);

        // Select and confirm arrays
        if self.select.is_empty() {
            bail!("At least one select level is required");
        }

        params.insert(
            "select",
            MAAValue::Array(self.select.into_iter().map(MAAValue::from).collect()),
        );

        params.insert(
            "confirm",
            MAAValue::Array(self.confirm.into_iter().map(MAAValue::from).collect()),
        );

        // First tags
        if !self.first_tags.is_empty() {
            params.insert(
                "first_tags",
                MAAValue::Array(self.first_tags.into_iter().map(MAAValue::from).collect()),
            );
        }

        // Extra tags mode validation
        if !(0..=2).contains(&self.extra_tags_mode) {
            bail!("extra_tags_mode must be between 0 and 2");
        }
        params.insert("extra_tags_mode", self.extra_tags_mode);

        // Times
        params.insert("times", self.times);

        // Set time only when times is 0
        if self.times == 0 {
            params.insert("set_time", self.set_time);
        }

        // Expedite
        if self.expedite {
            params.insert("expedite", true);
            params.maybe_insert("expedite_times", self.expedite_times);
        }

        params.insert("skip_robot", self.skip_robot);

        // Recruitment time
        if !self.recruitment_time.is_empty() {
            let mut time_map = std::collections::BTreeMap::new();

            for time_spec in self.recruitment_time {
                let mut parts = time_spec.split('=');
                let level = parts.next();
                let minutes = parts.next();

                match (level, minutes) {
                    (Some(level), Some(minutes)) => {
                        let level_str = level.to_owned();
                        let minutes: i32 = minutes.parse().with_context(|| {
                            format!("Failed to parse recruitment time minutes: {minutes}")
                        })?;

                        time_map.insert(level_str, minutes.into());
                    }
                    _ => {
                        bail!("Invalid recruitment time format: {}", time_spec)
                    }
                }
            }

            params.insert("recruitment_time", MAAValue::Object(time_map));
        }

        // Penguin Statistics reporting
        if self.report_to_penguin {
            params.insert("report_to_penguin", true);
            params.maybe_insert("penguin_id", self.penguin_id);
        }

        // Yituliu reporting
        if self.report_to_yituliu {
            params.insert("report_to_yituliu", true);
            params.maybe_insert("yituliu_id", self.yituliu_id);
        }

        // Server
        if let Some(server) = self.server {
            params.insert("server", server.server_report().unwrap_or("CN"));
        }

        Ok(params)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use maa_value::object;

    use super::*;
    use crate::command::{Command, parse_from};

    #[test]
    fn parse_recruit_params() {
        fn parse<I, T>(args: I) -> anyhow::Result<MAAValue>
        where
            I: IntoIterator<Item = T>,
            T: Into<std::ffi::OsString> + Clone,
        {
            let command = parse_from(args).command;
            match command {
                Command::Recruit { params, .. } => {
                    use super::super::{IntoParameters, TaskType, ToTaskType};
                    assert_eq!(params.to_task_type(), TaskType::Recruit);
                    params.into_parameters_no_context()
                }
                _ => panic!("Not a Recruit command"),
            }
        }

        // Test basic required parameters
        assert_eq!(
            parse(["maa", "recruit", "-s3", "-c3"]).unwrap(),
            object!(
                "refresh" => false,
                "select" => MAAValue::Array(vec![MAAValue::from(3)]),
                "confirm" => MAAValue::Array(vec![MAAValue::from(3)]),
                "extra_tags_mode" => 0,
                "times" => 0,
                "set_time" => true,
                "skip_robot" => true,
            )
        );

        // Test multiple select and confirm levels
        assert_eq!(
            parse(["maa", "recruit", "-s3", "-s4", "-c4"]).unwrap(),
            object!(
                "refresh" => false,
                "select" => MAAValue::Array(vec![MAAValue::from(3), MAAValue::from(4)]),
                "confirm" => MAAValue::Array(vec![MAAValue::from(4)]),
                "extra_tags_mode" => 0,
                "times" => 0,
                "set_time" => true,
                "skip_robot" => true,
            )
        );

        // Test with refresh and first tags
        assert_eq!(
            parse([
                "maa",
                "recruit",
                "-s3",
                "-c3",
                "--refresh",
                "-F控制",
                "-F削弱",
            ])
            .unwrap(),
            object!(
                "refresh" => true,
                "select" => MAAValue::Array(vec![MAAValue::from(3)]),
                "confirm" => MAAValue::Array(vec![MAAValue::from(3)]),
                "first_tags" => MAAValue::Array(vec![
                    MAAValue::from("控制"),
                    MAAValue::from("削弱"),
                ]),
                "extra_tags_mode" => 0,
                "times" => 0,
                "set_time" => true,
                "skip_robot" => true,
            )
        );

        // Test with times and expedite
        assert_eq!(
            parse([
                "maa",
                "recruit",
                "-s4",
                "-c4",
                "-t4",
                "--expedite",
                "--expedite-times=2",
            ])
            .unwrap(),
            object!(
                "refresh" => false,
                "select" => MAAValue::Array(vec![MAAValue::from(4)]),
                "confirm" => MAAValue::Array(vec![MAAValue::from(4)]),
                "extra_tags_mode" => 0,
                "times" => 4,
                "expedite" => true,
                "expedite_times" => 2,
                "skip_robot" => true,
            )
        );

        // Test with recruitment time
        assert_eq!(
            parse([
                "maa",
                "recruit",
                "-s3",
                "-s4",
                "-c4",
                "--recruitment-time=3=540",
                "--recruitment-time=4=460",
            ])
            .unwrap(),
            object!(
                "refresh" => false,
                "select" => MAAValue::Array(vec![MAAValue::from(3), MAAValue::from(4)]),
                "confirm" => MAAValue::Array(vec![MAAValue::from(4)]),
                "extra_tags_mode" => 0,
                "times" => 0,
                "set_time" => true,
                "skip_robot" => true,
                "recruitment_time" => object!(
                    "3" => 540,
                    "4" => 460,
                ),
            )
        );

        // Test with reporting
        assert_eq!(
            parse([
                "maa",
                "recruit",
                "-s3",
                "-c3",
                "--report-to-penguin",
                "--penguin-id=123",
                "--report-to-yituliu",
                "--yituliu-id=456",
                "--server=YoStarJP",
            ])
            .unwrap(),
            object!(
                "refresh" => false,
                "select" => MAAValue::Array(vec![MAAValue::from(3)]),
                "confirm" => MAAValue::Array(vec![MAAValue::from(3)]),
                "extra_tags_mode" => 0,
                "times" => 0,
                "set_time" => true,
                "skip_robot" => true,
                "report_to_penguin" => true,
                "penguin_id" => "123",
                "report_to_yituliu" => true,
                "yituliu_id" => "456",
                "server" => "JP",
            )
        );

        // Test extra_tags_mode
        assert_eq!(
            parse(["maa", "recruit", "-s3", "-c3", "--extra-tags-mode=2"]).unwrap(),
            object!(
                "refresh" => false,
                "select" => MAAValue::Array(vec![MAAValue::from(3)]),
                "confirm" => MAAValue::Array(vec![MAAValue::from(3)]),
                "extra_tags_mode" => 2,
                "times" => 0,
                "set_time" => true,
                "skip_robot" => true,
            )
        );

        // Test error cases
        assert!(parse(["maa", "recruit"]).is_err()); // Missing select
        assert!(parse(["maa", "recruit", "-s3", "-c3", "--extra-tags-mode=3"]).is_err()); // Invalid extra_tags_mode
        assert!(parse(["maa", "recruit", "-s3", "-c3", "--recruitment-time=invalid"]).is_err()); // Invalid recruitment time format
    }
}

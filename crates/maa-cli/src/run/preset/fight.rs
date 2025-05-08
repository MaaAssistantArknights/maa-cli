use anyhow::{bail, Context};

use super::MAAValue;
use crate::config::task::ClientType;

#[derive(clap::Args)]
pub struct FightParams {
    /// Stage to fight, e.g. 1-7, leave empty to fight current/last stage
    stage: Option<String>,
    #[clap(short, long)]
    /// Number of medicine (Sanity Potion) used to fight, default to 0
    medicine: Option<i32>,
    #[clap(long)]
    /// Number of expiring medicine (Sanity Potion) used to fight, default to 0
    expiring_medicine: Option<i32>,
    #[clap(long)]
    /// Number of stone (Originite Prime) used to fight, default to 0
    stone: Option<i32>,
    #[clap(long)]
    /// Exit after fighting given times, default to infinite
    times: Option<i32>,
    #[clap(short = 'D', long, action = clap::ArgAction::Append)]
    /// Exit after collecting given number of drops, default to no limit
    ///
    /// Example: `-D30012=100` to exit after get 100 Orirock Cube,
    /// 30012 is the item ID of Orirock Cube, you can find it at `item_index.json`.
    /// You can specify multiple drops, by repeating this option,
    /// e.g. `-D30012=100 -D30011=100` to exit after get 100 Orirock or 100 Orirock Cube.
    drops: Vec<String>,
    #[clap(long)]
    /// Repeat times of single proxy combat, 1 ~ 6, default to 1
    series: Option<i32>,
    #[clap(long)]
    /// Whether report drops to the Penguin Statistics
    report_to_penguin: bool,
    #[clap(long)]
    /// Penguin Statistics ID to report drops, leave empty to report anonymously
    penguin_id: Option<String>,
    #[clap(long)]
    /// Whether report drops to the yituliu
    report_to_yituliu: bool,
    #[clap(long)]
    /// Whether to report drops to the yituliu
    yituliu_id: Option<String>,
    #[clap(long)]
    /// Client type used to restart the game client if game crashed
    client_type: Option<ClientType>,
    #[clap(long)]
    /// Whether to use Originites like Dr. Grandet
    ///
    /// In DrGrandet mode, Wait in the using Originites confirmation screen until
    /// the 1 point of sanity has been restored and then immediately use the Originite.
    dr_grandet: bool,
}

impl super::ToTaskType for FightParams {
    fn to_task_type(&self) -> super::TaskType {
        super::TaskType::Fight
    }
}

impl TryFrom<FightParams> for MAAValue {
    type Error = anyhow::Error;

    fn try_from(args: FightParams) -> std::result::Result<Self, Self::Error> {
        let mut params = MAAValue::new();

        params.insert("stage", args.stage.unwrap_or_default());

        // Fight conditions
        params.maybe_insert("medicine", args.medicine);
        params.maybe_insert("expiring_medicine", args.expiring_medicine);
        params.maybe_insert("stone", args.stone);
        params.maybe_insert("times", args.times);

        let drops = args.drops;
        if !drops.is_empty() {
            let mut drop_map = std::collections::BTreeMap::new();

            for drop in drops {
                let mut parts = drop.split('=');
                let item_id = parts.next();
                let count = parts.next();

                match (item_id, count) {
                    (Some(item_id), Some(count)) => {
                        let count: i32 = count
                            .parse()
                            .with_context(|| format!(" Failed to parse drop count: {count}"))?;

                        drop_map.insert(item_id.to_owned(), count.into());
                    }
                    _ => {
                        bail!("Invalid drop format: {}", drop)
                    }
                }
            }

            params.insert("drops", MAAValue::Object(drop_map));
        }

        params.maybe_insert("series", args.series);

        if args.report_to_penguin {
            params.insert("report_to_penguin", true);
            params.maybe_insert("penguin_id", args.penguin_id);
        }

        if args.report_to_yituliu {
            params.insert("report_to_yituliu", true);
            params.maybe_insert("yituliu_id", args.yituliu_id);
        }

        if let Some(client_type) = args.client_type {
            params.insert("client_type", client_type.to_str());
            params.maybe_insert("server", client_type.server_report());
        }

        params.insert("DrGrandet", args.dr_grandet);

        Ok(params)
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use crate::{
        command::{parse_from, Command},
        object,
    };

    #[test]
    fn parse_fight_params() {
        fn parse<I, T>(args: I) -> anyhow::Result<MAAValue>
        where
            I: IntoIterator<Item = T>,
            T: Into<std::ffi::OsString> + Clone,
        {
            let command = parse_from(args).command;
            match command {
                Command::Fight { params, .. } => {
                    use super::super::{TaskType, ToTaskType};
                    assert_eq!(params.to_task_type(), TaskType::Fight);
                    params.try_into()
                }
                _ => panic!("Not a Fight command"),
            }
        }

        let default_params = object!(
            "stage" => "",
            "DrGrandet" => false,
        );

        assert_eq!(parse(["maa", "fight"]).unwrap(), default_params.clone());

        assert_eq!(
            parse([
                "maa",
                "fight",
                "1-7",
                "-m1",
                "-D30012=100",
                "--report-to-penguin",
                "--penguin-id=123456789",
                "--report-to-yituliu",
                "--yituliu-id=123456789",
                "--client-type=YoStarJP",
            ])
            .unwrap(),
            default_params.join(object!(
                "stage" => "1-7",
                "medicine" => 1,
                "drops" => object!("30012" => 100),
                "report_to_penguin" => true,
                "penguin_id" => "123456789",
                "report_to_yituliu" => true,
                "yituliu_id" => "123456789",
                "client_type" => "YoStarJP",
                "server" => "JP",
            ))
        );

        assert_eq!(
            parse([
                "maa",
                "fight",
                "1-7",
                "-m1",
                "-D30011=100",
                "-D30012=100",
                "--client-type=YoStarJP",
            ])
            .unwrap(),
            default_params.join(object!(
                "stage" => "1-7",
                "medicine" => 1,
                "drops" => object!(
                    "30011" => 100,
                    "30012" => 100,
                ),
                "client_type" => "YoStarJP",
                "server" => "JP",
            ))
        );

        assert_eq!(
            parse([
                "maa",
                "fight",
                "1-7",
                "--series=6",
                "--expiring-medicine=100",
                "--stone=10",
                "--dr-grandet",
            ])
            .unwrap(),
            object!(
                "stage" => "1-7",
                "expiring_medicine" => 100,
                "stone" => 10,
                "series" => 6,
                "DrGrandet" => true,
            )
        );

        assert!(parse(["maa", "fight", "1-7", "-D30012=100", "-D30011"]).is_err());
    }
}

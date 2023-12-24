use super::IterJoin;

use crate::config::task::task_type::{MAATask, TaskOrUnknown};

pub use std::collections::BTreeMap as Map;
use std::sync::Mutex;

use chrono;
use humantime::{self, format_duration};
use lazy_static::lazy_static;
use maa_sys::binding::AsstTaskId;

lazy_static! {
    static ref SUMMARY: Mutex<Option<Summary>> = Mutex::new(None);
}

// It's safe to unwarp the mutex all there, because lock() returns a error only when
// another thread failed inside the lock, which is impossible in this case, because
// there is no function that can panic inside the lock, unless the print!, which is
// not a problem.

pub fn init(task_summarys: Map<AsstTaskId, TaskSummary>) {
    *SUMMARY.lock().unwrap() = Some(Summary::new(task_summarys));
}

pub fn with_summary<T>(f: impl FnOnce(&Summary) -> T) -> Option<T> {
    SUMMARY.lock().unwrap().as_ref().map(f)
}

pub fn with_summary_mut<T>(f: impl FnOnce(&mut Summary) -> T) -> Option<T> {
    SUMMARY.lock().unwrap().as_mut().map(f)
}

// we print literal but it will be replace by a localizable string, so it's fine
#[allow(clippy::print_literal)]
pub(crate) fn display() -> Option<()> {
    with_summary(|summary| {
        if !summary.task_summarys.is_empty() {
            println!("# {}", "Summary");
            for (_, task_summary) in summary.task_summarys.iter() {
                if task_summary.has_started() {
                    println!("\n## {}", task_summary);
                }
            }
        }
    })
}

pub(super) fn start_task(id: AsstTaskId) -> Option<()> {
    with_summary_mut(|summary| summary.start_task(id)).flatten()
}

pub(super) fn end_current_task() -> Option<()> {
    with_summary_mut(|summary| summary.end_curent_task()).flatten()
}

pub(super) fn edit_current_task_detail(f: impl FnOnce(&mut Detail)) -> Option<()> {
    with_summary_mut(|summary| summary.edit_current_task_detail(f)).flatten()
}

pub struct Summary {
    task_summarys: Map<AsstTaskId, TaskSummary>,
    current_task: Option<AsstTaskId>,
}

impl Summary {
    pub fn new(task_summarys: Map<AsstTaskId, TaskSummary>) -> Self {
        Self {
            task_summarys,
            current_task: None,
        }
    }

    fn current_mut(&mut self) -> Option<&mut TaskSummary> {
        self.current_task
            .and_then(|id| self.task_summarys.get_mut(&id))
    }

    pub fn start_task(&mut self, id: AsstTaskId) -> Option<()> {
        self.task_summarys.get_mut(&id).map(|summary| {
            self.current_task = Some(id);
            summary.start();
        })
    }

    pub fn end_curent_task(&mut self) -> Option<()> {
        self.current_mut().map(|summary| summary.end()).map(|_| {
            self.current_task = None;
        })
    }

    pub fn edit_current_task_detail(&mut self, f: impl FnOnce(&mut Detail)) -> Option<()> {
        self.current_mut().map(|summary| summary.edit_detail(f))
    }
}

pub struct TaskSummary {
    task: TaskOrUnknown,
    detail: Detail,
    start_time: Option<chrono::DateTime<chrono::Local>>,
    end_time: Option<chrono::DateTime<chrono::Local>>,
}

impl TaskSummary {
    pub fn new(task: TaskOrUnknown) -> Self {
        use MAATask::*;

        let detail = match task {
            TaskOrUnknown::MAATask(Fight) => Detail::Fight(FightDetail::new()),
            TaskOrUnknown::MAATask(Infrast) => Detail::Infrast(InfrastDetail::new()),
            TaskOrUnknown::MAATask(Recruit) => Detail::Recruit(RecruitDetail::new()),
            TaskOrUnknown::MAATask(Roguelike) => Detail::Roguelike(RoguelikeDetail::new()),
            _ => Detail::None,
        };

        Self {
            task,
            detail,
            start_time: None,
            end_time: None,
        }
    }

    fn has_started(&self) -> bool {
        self.start_time.is_some() || self.end_time.is_some()
    }

    fn start(&mut self) {
        self.start_time = Some(chrono::Local::now());
    }

    fn end(&mut self) {
        self.end_time = Some(chrono::Local::now());
    }

    fn edit_detail(&mut self, f: impl FnOnce(&mut Detail)) {
        f(&mut self.detail);
    }
}

impl std::fmt::Display for TaskSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]", self.task)?;

        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => write!(
                f,
                " {} - {} ({})",
                start.format("%H:%M:%S"),
                end.format("%H:%M:%S"),
                format_duration((end - start).to_std().unwrap_or_default())
            ),
            (Some(start), None) => write!(f, " {} - ", start.format("%H:%M:%S")),
            (None, Some(end)) => write!(f, " - {}", end.format("%H:%M:%S")),
            (None, None) => Ok(()),
        }?;

        if !matches!(self.detail, Detail::None) {
            write!(f, "\n{}", self.detail)?;
        }

        Ok(())
    }
}

pub enum Detail {
    None,
    Infrast(InfrastDetail),
    Fight(FightDetail),
    Recruit(RecruitDetail),
    Roguelike(RoguelikeDetail),
}

impl Detail {
    pub fn as_infrast_mut(&mut self) -> Option<&mut InfrastDetail> {
        if let Detail::Infrast(detail) = self {
            Some(detail)
        } else {
            None
        }
    }

    pub fn as_fight_mut(&mut self) -> Option<&mut FightDetail> {
        if let Detail::Fight(detail) = self {
            Some(detail)
        } else {
            None
        }
    }

    pub fn as_recruit_mut(&mut self) -> Option<&mut RecruitDetail> {
        if let Detail::Recruit(detail) = self {
            Some(detail)
        } else {
            None
        }
    }

    pub fn as_roguelike_mut(&mut self) -> Option<&mut RoguelikeDetail> {
        if let Detail::Roguelike(detail) = self {
            Some(detail)
        } else {
            None
        }
    }
}

impl std::fmt::Display for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Detail::None => (),
            Detail::Fight(detail) => detail.fmt(f)?,
            Detail::Infrast(detail) => detail.fmt(f)?,
            Detail::Recruit(detail) => detail.fmt(f)?,
            Detail::Roguelike(detail) => detail.fmt(f)?,
        }

        Ok(())
    }
}

/// (facility) -> (id -> (info, operators, candidates))
pub struct InfrastDetail(Map<Facility, Map<i64, InfrastRoomInfo>>);

struct InfrastRoomInfo {
    info: Option<String>,
    operators: Vec<String>,
    candidates: Vec<String>,
}

impl InfrastRoomInfo {
    fn new_with_info(info: &str) -> Self {
        Self {
            info: Some(info.to_owned()),
            operators: Vec::new(),
            candidates: Vec::new(),
        }
    }

    fn new_with_operators(operators: &Vec<String>, candidates: &Vec<String>) -> Self {
        Self {
            info: None,
            operators: operators.to_owned(),
            candidates: candidates.to_owned(),
        }
    }

    fn set_info(&mut self, info: &str) {
        self.info = Some(info.to_owned());
    }

    fn set_operators(&mut self, operators: &Vec<String>, candidates: &Vec<String>) {
        self.operators = operators.to_owned();
        self.candidates = candidates.to_owned();
    }
}

impl std::fmt::Display for InfrastRoomInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(info) = self.info.as_ref() {
            write!(f, "({})", info)?;
        }
        write!(
            f,
            " operators: {}",
            self.operators.iter().join(", ", "none")
        )?;
        if !self.candidates.is_empty() {
            write!(
                f,
                " candidates: {}",
                self.candidates.iter().join(", ", "none")
            )?;
        }
        Ok(())
    }
}

impl InfrastDetail {
    pub fn new() -> Self {
        Self(Map::new())
    }

    pub(super) fn set_info(&mut self, facility: Facility, id: i64, info: &str) {
        self.0
            .entry(facility)
            .or_default()
            .entry(id)
            .and_modify(|room_info| room_info.set_info(info))
            .or_insert_with(|| InfrastRoomInfo::new_with_info(info));
    }

    pub(super) fn set_operators(
        &mut self,
        facility: Facility,
        id: i64,
        operators: &Vec<String>,
        candidates: &Vec<String>,
    ) {
        self.0
            .entry(facility)
            .or_default()
            .entry(id)
            .and_modify(|room_info| room_info.set_operators(operators, candidates))
            .or_insert_with(|| InfrastRoomInfo::new_with_operators(operators, candidates));
    }
}

impl std::fmt::Display for InfrastDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (facility, map) in &self.0 {
            for (id, room_info) in map {
                writeln!(f, "{} #{} {}", facility, id, room_info)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(super) enum Facility {
    Control,
    Mfg,
    Trade,
    Power,
    Office,
    Reception,
    Dorm,
    Processing,
    Training,
    Unknown,
}

impl Facility {
    fn to_str(self) -> &'static str {
        match self {
            Facility::Control => "Control",
            Facility::Mfg => "Mfg",
            Facility::Trade => "Trade",
            Facility::Power => "Power",
            Facility::Office => "Office",
            Facility::Reception => "Reception",
            Facility::Dorm => "Dorm",
            Facility::Processing => "Processing",
            Facility::Training => "Training",
            Facility::Unknown => "Unknown",
        }
    }
}

impl std::str::FromStr for Facility {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Control" => Ok(Facility::Control),
            "Mfg" => Ok(Facility::Mfg),
            "Trade" => Ok(Facility::Trade),
            "Power" => Ok(Facility::Power),
            "Office" => Ok(Facility::Office),
            "Reception" => Ok(Facility::Reception),
            "Dorm" => Ok(Facility::Dorm),
            "Processing" => Ok(Facility::Processing),
            "Training" => Ok(Facility::Training),
            _ => Ok(Facility::Unknown),
        }
    }
}

impl std::fmt::Display for Facility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

pub struct FightDetail {
    // stage name to fight
    stage: Option<String>,
    // times of fight
    times: Option<i64>,
    // used medicine
    medicine: Option<i64>,
    // used stone
    stone: Option<i64>,
    // [(item, count), ...], each element is corresponding to a fight
    // the length of this vector may smaller than times,
    // because some fight may not drop anything or failed to recognize the drop
    drops: Vec<Map<String, i64>>,
}

impl FightDetail {
    pub fn new() -> Self {
        Self {
            stage: None,
            times: None,
            medicine: None,
            stone: None,
            drops: Vec::new(),
        }
    }

    pub fn set_stage(&mut self, stage: &str) {
        if self.stage.is_some() {
            return;
        }
        self.stage = Some(stage.to_owned());
    }

    pub fn set_times(&mut self, times: i64) {
        self.times = Some(times);
    }

    pub fn set_medicine(&mut self, medicine: i64) {
        self.medicine = Some(medicine);
    }

    pub fn set_stone(&mut self, stone: i64) {
        self.stone = Some(stone);
    }

    pub fn push_drop(&mut self, drop: Map<String, i64>) {
        self.drops.push(drop);
    }
}

impl std::fmt::Display for FightDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(times) = self.times {
            write!(
                f,
                "Fight {} {} times",
                self.stage
                    .as_ref()
                    .map_or("unknown", |stage| stage.as_str()),
                times
            )?;
        } else {
            return Ok(());
        }
        if let Some(medicine) = self.medicine {
            write!(f, ", used {} medicine", medicine)?;
        }
        if let Some(stone) = self.stone {
            write!(f, ", used {} stone", stone)?;
        }
        if !self.drops.is_empty() {
            writeln!(f, ", drops:")?;
            let mut total_drop = Map::new();
            for drop in self.drops.iter() {
                write!(f, "-")?;
                if drop.is_empty() {
                    write!(f, " no drop or unrecognized")?;
                }
                for (item, count) in drop {
                    write!(f, " {} × {}", item, count)?;
                    insert_or_add_by_ref(&mut total_drop, item, *count);
                }
                writeln!(f)?;
            }
            write!(f, "total:")?;
            for (item, count) in total_drop {
                write!(f, " {} × {}", item, count)?;
            }
        }
        Ok(())
    }
}

pub struct RecruitDetail {
    refresh_times: Option<i64>,
    recruit_times: Option<i64>,
    // [(tags, level, state), ...]
    record: Vec<(u64, Vec<String>, RecruitState)>,
}

enum RecruitState {
    Refreshed,
    Recruited,
    None,
}

impl RecruitDetail {
    pub fn new() -> Self {
        Self {
            refresh_times: None,
            recruit_times: None,
            record: Vec::new(),
        }
    }

    pub(super) fn refresh(&mut self) {
        self.refresh_times = Some(self.refresh_times.unwrap_or_default() + 1);
        if let Some((_, _, state)) = self.record.last_mut() {
            *state = RecruitState::Refreshed;
        }
    }

    pub(super) fn recruit(&mut self) {
        self.recruit_times = Some(self.recruit_times.unwrap_or_default() + 1);
        if let Some((_, _, state)) = self.record.last_mut() {
            *state = RecruitState::Recruited;
        }
    }

    pub(super) fn push_recruit(&mut self, level: u64, tags: impl IntoIterator<Item = String>) {
        self.record
            .push((level, tags.into_iter().collect(), RecruitState::None));
    }
}

impl std::fmt::Display for RecruitDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Recruit:")?;
        if let Some(times) = self.recruit_times {
            writeln!(f, ", recruit {} times", times)?;
        }
        if let Some(times) = self.refresh_times {
            writeln!(f, ", refresh {} times", times)?;
        }
        if !self.record.is_empty() {
            writeln!(f, ", detected tags:")?;
            for (level, tags, state) in self.record.iter() {
                write!(
                    f,
                    "- {} {}",
                    "★".repeat(*level as usize),
                    tags.iter().join(", ", "unknown"),
                )?;
                match state {
                    RecruitState::Refreshed => write!(f, " refreshed;")?,
                    RecruitState::Recruited => write!(f, " recruited;")?,
                    RecruitState::None => (),
                }
                writeln!(f)?
            }
        }
        Ok(())
    }
}

pub struct RoguelikeDetail {
    times: Option<i64>,
    invest: Option<i64>,
}

impl RoguelikeDetail {
    pub fn new() -> Self {
        Self {
            times: None,
            invest: None,
        }
    }

    pub fn set_times(&mut self, times: i64) {
        self.times = Some(times);
    }

    pub fn set_invest(&mut self, invest: i64) {
        self.invest = Some(invest);
    }
}

impl std::fmt::Display for RoguelikeDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(times) = self.times {
            write!(f, "Explore {} times", times)?;
        }
        if let Some(invest) = self.invest {
            if invest > 0 {
                write!(f, " invest {} times", invest)?;
            }
        }
        Ok(())
    }
}

pub fn insert_or_add_by_ref(map: &mut Map<String, i64>, key: &str, value: i64) {
    if let Some(old) = map.get_mut(key) {
        *old += value;
    } else {
        map.insert(key.to_owned(), value);
    }
}

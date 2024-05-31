use super::IterJoin;

pub use std::collections::BTreeMap as Map;
use std::sync::Mutex;

use chrono;
use maa_sys::{binding::AsstTaskId, TaskType};

static SUMMARY: Mutex<Option<Summary>> = Mutex::new(None);

// It's safe to unwarp the mutex all there, because lock() returns a error only when
// another thread failed inside the lock, which is impossible in this case, because
// there is no function that can panic inside the lock, unless the print!, which is
// not a problem.

pub(crate) fn init(summary: Summary) {
    *SUMMARY.lock().unwrap() = Some(summary);
}

fn with_summary<T>(f: impl FnOnce(&Summary) -> T) -> Option<T> {
    SUMMARY.lock().unwrap().as_ref().map(f)
}

fn with_summary_mut<T>(f: impl FnOnce(&mut Summary) -> T) -> Option<T> {
    SUMMARY.lock().unwrap().as_mut().map(f)
}

pub(crate) fn display() -> Option<()> {
    with_summary(|summary| print!("{}", summary))
}

pub(super) fn start_task(id: AsstTaskId) -> Option<()> {
    with_summary_mut(|summary| summary.start_task(id)).flatten()
}

pub(super) fn end_current_task(reason: Reason) -> Option<()> {
    with_summary_mut(|summary| summary.end_current_task(reason)).flatten()
}

pub(super) fn edit_current_task_detail(f: impl FnOnce(&mut Detail)) -> Option<()> {
    with_summary_mut(|summary| summary.edit_current_task_detail(f)).flatten()
}

pub struct Summary {
    task_summarys: Map<AsstTaskId, TaskSummary>,
    current_task: Option<AsstTaskId>,
}

impl Summary {
    pub fn new() -> Self {
        Self {
            task_summarys: Map::new(),
            current_task: None,
        }
    }

    pub fn insert(&mut self, id: AsstTaskId, name: Option<String>, task: impl Into<TaskType>) {
        self.task_summarys
            .insert(id, TaskSummary::new(name, task.into()));
    }

    fn current_mut(&mut self) -> Option<&mut TaskSummary> {
        self.current_task
            .and_then(|id| self.task_summarys.get_mut(&id))
    }

    fn start_task(&mut self, id: AsstTaskId) -> Option<()> {
        self.task_summarys.get_mut(&id).map(|summary| {
            self.current_task = Some(id);
            summary.start();
        })
    }

    fn end_current_task(&mut self, reason: Reason) -> Option<()> {
        self.current_mut()
            .map(|summary| summary.end(reason))
            .map(|_| self.current_task = None)
    }

    fn edit_current_task_detail(&mut self, f: impl FnOnce(&mut Detail)) -> Option<()> {
        self.current_mut().map(|summary| summary.edit_detail(f))
    }
}

const LINE_SEP: &str = "----------------------------------------";

impl std::fmt::Display for Summary {
    // we print literal but it will be replace by a localizable string, so it's fine
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.task_summarys.is_empty() {
            #[allow(clippy::write_literal)]
            writeln!(f, "{}", "Summary")?;
            for task_summary in self.task_summarys.values() {
                write!(f, "{LINE_SEP}\n{task_summary}")?;
            }
        }
        Ok(())
    }
}

pub struct TaskSummary {
    name: Option<String>,
    task: TaskType,
    detail: Detail,
    start_time: Option<chrono::DateTime<chrono::Local>>,
    end_time: Option<chrono::DateTime<chrono::Local>>,
    reason: Reason,
}

impl TaskSummary {
    pub fn new(name: Option<String>, task: TaskType) -> Self {
        use TaskType::*;

        let detail = match task {
            Fight => Detail::Fight(FightDetail::new()),
            Infrast => Detail::Infrast(InfrastDetail::new()),
            Recruit => Detail::Recruit(RecruitDetail::new()),
            Roguelike => Detail::Roguelike(RoguelikeDetail::new()),
            Depot => Detail::Depot(DepotDetail::new()),
            OperBox => Detail::OperBox(OperBoxDetail::new()),
            _ => Detail::None,
        };

        Self {
            name,
            task,
            detail,
            start_time: None,
            end_time: None,
            reason: Reason::Unstarted,
        }
    }

    fn start(&mut self) {
        self.start_time = Some(chrono::Local::now());
        self.reason = Reason::Unfinished;
    }

    fn end(&mut self, reason: Reason) {
        self.end_time = Some(chrono::Local::now());
        self.reason = reason;
    }

    fn edit_detail(&mut self, f: impl FnOnce(&mut Detail)) {
        f(&mut self.detail);
    }
}

impl std::fmt::Display for TaskSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            self.name.as_deref().unwrap_or(self.task.as_ref())
        )?;

        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => write!(
                f,
                " {} - {} ({})",
                start.format("%H:%M:%S"),
                end.format("%H:%M:%S"),
                FormattedDuration::from(end - start)
            ),
            (Some(start), None) => write!(f, " {} -", start.format("%H:%M:%S")),
            (None, Some(end)) => write!(f, " - {}", end.format("%H:%M:%S")),
            (None, None) => Ok(()),
        }?;

        match self.reason {
            Reason::Completed => write!(f, " Completed")?,
            Reason::Stopped => write!(f, " Stopped")?,
            Reason::Error => write!(f, " Error")?,
            Reason::Unfinished => write!(f, " Unfinished")?,
            Reason::Unstarted => write!(f, " Unstarted")?,
        }

        writeln!(f)?;

        if !matches!(self.detail, Detail::None) {
            write!(f, "{}", self.detail)?;
        }

        Ok(())
    }
}

pub(super) enum Reason {
    Completed,
    Stopped,
    Error,
    Unstarted,
    Unfinished,
}

struct FormattedDuration {
    hours: i64,
    minutes: i64,
    seconds: i64,
}

impl From<chrono::Duration> for FormattedDuration {
    fn from(duration: chrono::Duration) -> Self {
        let total_seconds = duration.num_seconds();

        let hours = total_seconds / (60 * 60);
        let minutes = (total_seconds / 60) % 60;
        let seconds = total_seconds % 60;

        Self {
            hours,
            minutes,
            seconds,
        }
    }
}

impl std::fmt::Display for FormattedDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut is_first = true;
        if self.hours > 0 {
            is_first = false;
            write!(f, "{}h", self.hours)?;
        }
        if self.minutes > 0 {
            if !is_first {
                write!(f, " ")?;
            } else {
                is_first = false;
            }
            write!(f, "{}m", self.minutes)?;
        }
        if is_first {
            write!(f, "{}s", self.seconds)?;
        } else if self.seconds > 0 {
            write!(f, " {}s", self.seconds)?;
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
    Depot(DepotDetail),
    OperBox(OperBoxDetail),
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

    pub fn as_depot_mut(&mut self) -> Option<&mut DepotDetail> {
        if let Detail::Depot(detail) = self {
            Some(detail)
        } else {
            None
        }
    }

    pub fn as_operbox_mut(&mut self) -> Option<&mut OperBoxDetail> {
        if let Detail::OperBox(detail) = self {
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
            Detail::Depot(detail) => detail.fmt(f)?,
            Detail::OperBox(detail) => detail.fmt(f)?,
        }

        Ok(())
    }
}

pub struct InfrastDetail(Map<Facility, Map<i64, InfrastRoomInfo>>);

struct InfrastRoomInfo {
    product: Option<String>,
    operators: Vec<String>,
    candidates: Vec<String>,
}

impl InfrastDetail {
    pub fn new() -> Self {
        Self(Map::new())
    }

    pub(super) fn set_product(&mut self, facility: Facility, id: i64, info: &str) {
        use Facility::*;
        // only the product of Mfg and Trade is useful
        if matches!(facility, Mfg | Trade) {
            self.0
                .entry(facility)
                .or_default()
                .entry(id)
                .and_modify(|room_info| room_info.set_product(info))
                .or_insert_with(|| InfrastRoomInfo::new_with_info(info));
        }
    }

    pub(super) fn set_operators(
        &mut self,
        facility: Facility,
        id: i64,
        operators: Vec<String>,
        candidates: Vec<String>,
    ) {
        let map = self.0.entry(facility).or_default();

        if let Some(room_info) = map.get_mut(&id) {
            room_info.set_operators(operators, candidates);
        } else {
            map.insert(
                id,
                InfrastRoomInfo::new_with_operators(operators, candidates),
            );
        }
    }
}

impl std::fmt::Display for InfrastDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (facility, map) in &self.0 {
            for room_info in map.values() {
                writeln!(f, "{}{}", facility, room_info)?;
            }
        }

        Ok(())
    }
}

#[cfg_attr(test, derive(Debug))]
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
        use Facility::*;
        match self {
            Control => "Control",
            Mfg => "Mfg",
            Trade => "Trade",
            Power => "Power",
            Office => "Office",
            Reception => "Reception",
            Dorm => "Dorm",
            Processing => "Processing",
            Training => "Training",
            Unknown => "Unknown",
        }
    }
}

impl std::str::FromStr for Facility {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Facility::*;
        match s {
            "Control" => Ok(Control),
            "Mfg" => Ok(Mfg),
            "Trade" => Ok(Trade),
            "Power" => Ok(Power),
            "Office" => Ok(Office),
            "Reception" => Ok(Reception),
            "Dorm" => Ok(Dorm),
            "Processing" => Ok(Processing),
            "Training" => Ok(Training),
            _ => Ok(Unknown),
        }
    }
}

impl std::fmt::Display for Facility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_str())
    }
}

impl InfrastRoomInfo {
    fn new_with_info(info: &str) -> Self {
        Self {
            product: Some(info.to_owned()),
            operators: Vec::new(),
            candidates: Vec::new(),
        }
    }

    fn new_with_operators(operators: Vec<String>, candidates: Vec<String>) -> Self {
        Self {
            product: None,
            operators,
            candidates,
        }
    }

    fn set_product(&mut self, product: &str) {
        self.product = Some(product.to_owned());
    }

    fn set_operators(&mut self, operators: Vec<String>, candidates: Vec<String>) {
        self.operators = operators;
        self.candidates = candidates;
    }
}

impl std::fmt::Display for InfrastRoomInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(info) = self.product.as_ref() {
            write!(f, "({})", info)?;
        }
        write!(
            f,
            " with operators: {}",
            self.operators
                .iter()
                .join(", ")
                .unwrap_or_else(|| "unknown".to_owned())
        )?;
        if !self.candidates.is_empty() {
            write!(
                f,
                ", [{}]",
                self.candidates.iter().join(", ").unwrap() // safe to unwrap, because it's not empty
            )?;
        }
        Ok(())
    }
}

pub struct FightDetail {
    // stage name to fight
    stage: Option<String>,
    // times of fight
    times: Option<i64>,
    // used medicine
    medicine: Option<(i64, i64)>,
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

    pub fn use_medicine(&mut self, count: i64, is_expiring: bool) {
        let (mut all, mut expiring) = self.medicine.unwrap_or((0, 0));
        all += count;
        if is_expiring {
            expiring += count
        }
        self.medicine = Some((all, expiring))
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
        if let Some(stage) = self.stage.as_ref() {
            write!(f, "Fight {stage}")?;
        } else {
            return Ok(());
        }

        if let Some(times) = self.times {
            write!(f, " {times} times")?;
        }
        if let Some((all, expiring)) = self.medicine {
            write!(f, ", used {all} medicine ({expiring} expiring)")?;
        }
        if let Some(stone) = self.stone {
            write!(f, ", used {stone} stone")?;
        }
        if !self.drops.is_empty() {
            writeln!(f, ", drops:")?;
            let mut total_drop = Map::new();
            for (i, drop) in self.drops.iter().enumerate() {
                write!(f, "{}.", i + 1)?;
                let mut iter = drop.iter();
                if let Some((item, count)) = iter.next() {
                    write!(f, " {} × {}", item, count)?;
                    insert_or_add_by_ref(&mut total_drop, item, *count);
                }
                for (item, count) in iter {
                    write!(f, ", {} × {}", item, count)?;
                    insert_or_add_by_ref(&mut total_drop, item, *count);
                }
                writeln!(f)?;
            }
            write!(f, "total drops:")?;
            let mut iter = total_drop.iter();
            if let Some((item, count)) = iter.next() {
                write!(f, " {} × {}", item, count)?;
            }
            for (item, count) in iter {
                write!(f, ", {} × {}", item, count)?;
            }
        }
        writeln!(f)?;
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
        if !self.record.is_empty() {
            writeln!(f, "Detected tags:")?;
            for (i, (level, tags, state)) in self.record.iter().enumerate() {
                write!(
                    f,
                    "{}. {} {}",
                    i + 1,
                    "★".repeat(*level as usize),
                    tags.iter()
                        .join(", ")
                        .unwrap_or_else(|| "unknown".to_owned())
                )?;
                match state {
                    RecruitState::Refreshed => write!(f, ", Refreshed")?,
                    RecruitState::Recruited => write!(f, ", Recruited")?,
                    RecruitState::None => (),
                }
                writeln!(f)?
            }
            if let Some(times) = self.recruit_times {
                writeln!(f, "Recruited {} times", times)?;
            }
            if let Some(times) = self.refresh_times {
                writeln!(f, "Refreshed {} times", times)?;
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
            if let Some(invest) = self.invest {
                if invest > 0 {
                    write!(f, " invest {} times", invest)?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub struct DepotDetail(Option<Map<String, i64>>);

impl DepotDetail {
    pub fn new() -> Self {
        Self(None)
    }

    pub fn set_depot(&mut self, map: Map<String, i64>) {
        self.0 = Some(map);
    }
}

impl std::fmt::Display for DepotDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(map) = &self.0 {
            writeln!(f, "ITEM\tCOUNT")?;
            for (item, count) in map {
                writeln!(f, "{}\t{}", item, count)?;
            }
        }
        Ok(())
    }
}

pub struct OperBoxDetail(Vec<Map<String, Option<OperInfo>>>);

impl OperBoxDetail {
    pub fn new() -> Self {
        Self(Vec::with_capacity(6))
    }

    pub fn set_operbox(&mut self, operbox: Vec<Map<String, Option<OperInfo>>>) {
        self.0.extend(operbox);
    }
}

impl std::fmt::Display for OperBoxDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "NAME\tPOTENTIAL\tELITE\tLEVEL")?;
        for map in self.0.iter().rev() {
            if !map.is_empty() {
                for (name, info) in map {
                    write!(f, "{}\t", name)?;
                    if let Some(info) = info {
                        write!(f, "{}\t{}\t{}", info.potential, info.elite, info.level)?;
                    } else {
                        write!(f, "\t\t")?;
                    }
                    writeln!(f)?;
                }
            }
        }
        Ok(())
    }
}

pub struct OperInfo {
    pub(super) potential: i64,
    pub(super) elite: i64,
    pub(super) level: i64,
}

pub fn insert_or_add_by_ref(map: &mut Map<String, i64>, key: &str, value: i64) {
    if let Some(old) = map.get_mut(key) {
        *old += value;
    } else {
        map.insert(key.to_owned(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration() {
        fn seconds(s: i64) -> chrono::Duration {
            chrono::TimeDelta::try_seconds(s).unwrap()
        }

        assert_eq!(FormattedDuration::from(seconds(0)).to_string(), "0s");
        assert_eq!(FormattedDuration::from(seconds(1)).to_string(), "1s");
        assert_eq!(FormattedDuration::from(seconds(60)).to_string(), "1m");
        assert_eq!(FormattedDuration::from(seconds(60 * 60)).to_string(), "1h");
        assert_eq!(
            FormattedDuration::from(seconds(60 * 60 + 1)).to_string(),
            "1h 1s"
        );
        assert_eq!(
            FormattedDuration::from(seconds(60 * 60 + 60)).to_string(),
            "1h 1m"
        );
        assert_eq!(
            FormattedDuration::from(seconds(60 * 60 + 60 + 1)).to_string(),
            "1h 1m 1s"
        );
        assert_eq!(
            FormattedDuration::from(seconds(60 * 60 * 48)).to_string(),
            "48h"
        );
    }

    mod summary {
        use regex::Regex;

        use super::*;

        use crate::assert_matches;

        #[test]
        fn task_summary() {
            use TaskType::*;

            let mut summary = Summary::new();
            summary.insert(1, Some("Fight TS".to_owned()), Fight);
            summary.insert(2, None, Infrast);
            summary.insert(3, None, Recruit);
            summary.insert(4, None, Roguelike);
            summary.insert(5, None, CloseDown);

            summary.start_task(1);
            summary.edit_current_task_detail(|detail| {
                let detail = detail.as_fight_mut().unwrap();
                detail.set_stage("TS-9");
            });
            summary.end_current_task(Reason::Completed);

            summary.start_task(2);
            summary.edit_current_task_detail(|detail| {
                let detail = detail.as_infrast_mut().unwrap();
                detail.set_product(Facility::Mfg, 1, "Product");
            });
            summary.end_current_task(Reason::Stopped);

            summary.start_task(3);
            summary.edit_current_task_detail(|detail| {
                let detail = detail.as_recruit_mut().unwrap();
                detail.push_recruit(3, ["A", "B"].into_iter().map(|s| s.to_owned()));
                detail.recruit();
            });
            summary.end_current_task(Reason::Error);

            summary.start_task(4);

            let task1 = summary.task_summarys.get(&1).unwrap();
            assert!(task1.start_time.is_some());
            assert!(task1.end_time.is_some());
            assert_matches!(task1.reason, Reason::Completed);

            let task2 = summary.task_summarys.get(&2).unwrap();
            assert!(task2.start_time.is_some());
            assert!(task2.end_time.is_some());
            assert_matches!(task2.reason, Reason::Stopped);

            let task3 = summary.task_summarys.get(&3).unwrap();
            assert!(task3.start_time.is_some());
            assert!(task3.end_time.is_some());
            assert_matches!(task3.reason, Reason::Error);

            let task4 = summary.task_summarys.get(&4).unwrap();
            assert!(task4.start_time.is_some());
            assert!(task4.end_time.is_none());
            assert_matches!(task4.reason, Reason::Unfinished);

            let task5 = summary.task_summarys.get(&5).unwrap();
            assert!(task5.start_time.is_none());
            assert!(task5.end_time.is_none());
            assert_matches!(task5.reason, Reason::Unstarted);

            let re = Regex::new(
                "Summary\n\
                ----------------------------------------\n\
                \\[Fight TS\\] \\d+:\\d+:\\d+ - \\d+:\\d+:\\d+ \\(\\d+s\\) Completed\n\
                .+\n\
                ----------------------------------------\n\
                \\[Infrast\\] \\d+:\\d+:\\d+ - \\d+:\\d+:\\d+ \\(\\d+s\\) Stopped\n\
                .+\n\
                ----------------------------------------\n\
                \\[Recruit\\] \\d+:\\d+:\\d+ - \\d+:\\d+:\\d+ \\(\\d+s\\) Error\n\
                .+\n.+\n.+\n\
                ----------------------------------------\n\
                \\[Roguelike\\] \\d+:\\d+:\\d+ - Unfinished\n\
                ----------------------------------------\n\
                \\[CloseDown\\] Unstarted\n",
            )
            .unwrap();

            assert!(re.is_match(&summary.to_string()));
        }
    }

    mod detail {
        use super::*;

        #[test]
        fn detail() {
            let mut detail = Detail::None;
            assert!(detail.as_infrast_mut().is_none());
            assert!(detail.as_fight_mut().is_none());
            assert!(detail.as_recruit_mut().is_none());
            assert!(detail.as_roguelike_mut().is_none());

            detail = Detail::Infrast(InfrastDetail::new());
            assert!(detail.as_infrast_mut().is_some());
            assert!(detail.as_fight_mut().is_none());
            assert!(detail.as_recruit_mut().is_none());
            assert!(detail.as_roguelike_mut().is_none());

            detail = Detail::Fight(FightDetail::new());
            assert!(detail.as_infrast_mut().is_none());
            assert!(detail.as_fight_mut().is_some());
            assert!(detail.as_recruit_mut().is_none());
            assert!(detail.as_roguelike_mut().is_none());

            detail = Detail::Recruit(RecruitDetail::new());
            assert!(detail.as_infrast_mut().is_none());
            assert!(detail.as_fight_mut().is_none());
            assert!(detail.as_recruit_mut().is_some());
            assert!(detail.as_roguelike_mut().is_none());

            detail = Detail::Roguelike(RoguelikeDetail::new());
            assert!(detail.as_infrast_mut().is_none());
            assert!(detail.as_fight_mut().is_none());
            assert!(detail.as_recruit_mut().is_none());
            assert!(detail.as_roguelike_mut().is_some());
        }

        #[test]
        fn facility() {
            use Facility::*;
            assert_eq!(Control.to_string(), "Control");
            assert_eq!(Mfg.to_string(), "Mfg");
            assert_eq!(Trade.to_string(), "Trade");
            assert_eq!(Power.to_string(), "Power");
            assert_eq!(Office.to_string(), "Office");
            assert_eq!(Reception.to_string(), "Reception");
            assert_eq!(Dorm.to_string(), "Dorm");
            assert_eq!(Processing.to_string(), "Processing");
            assert_eq!(Training.to_string(), "Training");
            assert_eq!(Unknown.to_string(), "Unknown");

            assert_eq!("Control".parse::<Facility>().unwrap(), Control);
            assert_eq!("Mfg".parse::<Facility>().unwrap(), Mfg);
            assert_eq!("Trade".parse::<Facility>().unwrap(), Trade);
            assert_eq!("Power".parse::<Facility>().unwrap(), Power);
            assert_eq!("Office".parse::<Facility>().unwrap(), Office);
            assert_eq!("Reception".parse::<Facility>().unwrap(), Reception);
            assert_eq!("Dorm".parse::<Facility>().unwrap(), Dorm);
            assert_eq!("Processing".parse::<Facility>().unwrap(), Processing);
            assert_eq!("Training".parse::<Facility>().unwrap(), Training);
            assert_eq!("Unknown".parse::<Facility>().unwrap(), Unknown);
            assert_eq!("Other".parse::<Facility>().unwrap(), Unknown);
        }

        #[test]
        fn infrast() {
            let mut detail = InfrastDetail::new();
            detail.set_product(Facility::Mfg, 1, "Product");
            detail.set_operators(
                Facility::Mfg,
                1,
                ["A", "B"].into_iter().map(|s| s.to_owned()).collect(),
                ["C", "D"].into_iter().map(|s| s.to_owned()).collect(),
            );
            detail.set_product(Facility::Office, 1, "Product");
            detail.set_operators(Facility::Office, 1, Vec::new(), Vec::new());
            assert_eq!(
                detail.to_string(),
                "Mfg(Product) with operators: A, B, [C, D]\n\
                 Office with operators: unknown\n",
            );
        }

        #[test]
        fn fight() {
            let mut detail = FightDetail::new();
            detail.set_stage("TS-9");
            detail.set_times(2);
            detail.use_medicine(1, true);
            detail.use_medicine(1, false);
            detail.set_stone(1);
            detail.push_drop(
                [("A", 1), ("B", 2)]
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), v))
                    .collect(),
            );
            detail.push_drop(
                [("A", 1), ("C", 3)]
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), v))
                    .collect(),
            );
            assert_eq!(
                detail.to_string(),
                "Fight TS-9 2 times, used 2 medicine (1 expiring), used 1 stone, drops:\n\
                 1. A × 1, B × 2\n\
                 2. A × 1, C × 3\n\
                 total drops: A × 2, B × 2, C × 3\n",
            );

            let mut detail = FightDetail::new();
            detail.set_stage("TS-9");
            detail.set_times(1);
            assert_eq!(detail.to_string(), "Fight TS-9 1 times\n");

            let detail = FightDetail::new();
            assert_eq!(detail.to_string(), "");
        }

        #[test]
        fn recruit() {
            let mut detail = RecruitDetail::new();
            detail.push_recruit(3, ["A", "B"].into_iter().map(|s| s.to_owned()));
            detail.refresh();
            detail.push_recruit(4, ["C", "D"].into_iter().map(|s| s.to_owned()));
            detail.recruit();
            detail.push_recruit(3, ["E", "F"].into_iter().map(|s| s.to_owned()));
            detail.refresh();
            detail.push_recruit(4, ["G", "H"].into_iter().map(|s| s.to_owned()));
            detail.recruit();
            detail.push_recruit(5, ["I", "J"].into_iter().map(|s| s.to_owned()));
            assert_eq!(
                detail.to_string(),
                "Detected tags:\n\
                 1. ★★★ A, B, Refreshed\n\
                 2. ★★★★ C, D, Recruited\n\
                 3. ★★★ E, F, Refreshed\n\
                 4. ★★★★ G, H, Recruited\n\
                 5. ★★★★★ I, J\n\
                 Recruited 2 times\n\
                 Refreshed 2 times\n",
            );

            let mut detail = RecruitDetail::new();
            detail.push_recruit(3, ["A", "B"].into_iter().map(|s| s.to_owned()));
            detail.recruit();
            assert_eq!(
                detail.to_string(),
                "Detected tags:\n\
                 1. ★★★ A, B, Recruited\n\
                 Recruited 1 times\n",
            );
            let mut detail = RecruitDetail::new();
            detail.push_recruit(3, ["A", "B"].into_iter().map(|s| s.to_owned()));
            detail.refresh();
            detail.push_recruit(4, ["C", "D"].into_iter().map(|s| s.to_owned()));
            assert_eq!(
                detail.to_string(),
                "Detected tags:\n\
                 1. ★★★ A, B, Refreshed\n\
                 2. ★★★★ C, D\n\
                 Refreshed 1 times\n",
            );
        }

        #[test]
        fn roguelike() {
            let mut detail = RoguelikeDetail::new();
            detail.set_times(2);
            detail.set_invest(1);
            assert_eq!(detail.to_string(), "Explore 2 times invest 1 times\n");
        }

        #[test]
        fn depot() {
            let mut detail = DepotDetail::new();
            detail.set_depot(
                [("A", 1), ("B", 2)]
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), v))
                    .collect(),
            );
            assert_eq!(detail.to_string(), "ITEM\tCOUNT\nA\t1\nB\t2\n");
        }

        #[test]
        fn operbox() {
            let mut detail = OperBoxDetail::new();
            detail.set_operbox(vec![
                [(
                    "A",
                    Some(OperInfo {
                        potential: 1,
                        elite: 0,
                        level: 1,
                    }),
                )]
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
                [("B", None)]
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), v))
                    .collect(),
            ]);
            assert_eq!(
                detail.to_string(),
                "NAME\tPOTENTIAL\tELITE\tLEVEL\n\
                 B\t\t\t\n\
                 A\t1\t0\t1\n\
                ",
            );
        }
    }
}

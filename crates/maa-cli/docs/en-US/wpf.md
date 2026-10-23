# Migrate a WPF GUI profile

`maa migrate wpf` turns a JSON export from the MAA Windows GUI into a maa-cli task file.
Unsupported tasks and settings are skipped.
When the command finishes, it writes a summary to stderr.
Check the result against [Configuration](config.md) before you use it for daily runs.

## Migrate one profile

The input is the JSON file exported from the GUI.

```bash
# Write a .toml file with the same name as the input
maa migrate wpf gui.json

# The extension picks the format: toml, yaml, or json
maa migrate wpf gui.json tasks/daily.toml

# Pick one profile by name when the export contains several
maa migrate wpf gui.json --profile-name Default

# Use your schedule file when the GUI profile already uses a custom base schedule
maa migrate wpf gui.json --custom-schedule ~/base-schedule.json
```

If the export contains one profile, the command uses it.
If it contains several and you omit `--profile-name`, the command asks you to choose.

`--custom-schedule` works only when that infrast task is already a custom schedule in the GUI.
If it is not, the option makes the command exit with an error.

Put the generated file in `$MAA_CONFIG_DIR/tasks`, then run it with `maa run <task>`.
Find the config directory with `maa dir config`.

## Settings already described in the configuration docs

The output has the same shape as a [custom task](config.md#custom-tasks).
The top level is `[[tasks]]`.
Each task has `type`, `name`, and `params`.
It also has `variants` when a condition selects among stages or shifts.
Condition syntax is in [task variants and conditions](config.md#task-variants-and-conditions).

The names below are ones the configuration docs already use.
Names the configuration docs do not explain, which MaaCore reads at run time, are in the next section.

### Start up

`client_type` and `start_game_enabled` are the two fields in the start-up example in the configuration docs.
If the GUI has no recognized client, `client_type` becomes a prompt you answer when the task runs.
That prompt is the user-input form in the configuration docs.
If account switching is on and the account name is empty, that setting is not written.
The summary then has one skipped field.

### Fight

Each stage is a variant.
The stage name is `stage`.

The condition follows the stage kind.

- A resource or chip stage uses `Weekday` in the official server timezone.
- An event stage uses `DateTime` when this machine's hot-update event table has that event. The condition has a start, an end, and a timezone. Otherwise it uses `Always`, and the command warns.
- A permanent stage, such as a main story stage, annihilation, or `LS-6`, uses `Always`.

`DateTime`, `Weekday`, and `DayMod` are in the configuration docs.
`Always` is also a maa-cli condition. It ignores the clock, so the variant can always run.
`start` and `end` for an event stage come from the event table on this machine at migration time.
They do not follow a later hot update.
Migrate again, or edit the condition, after the event changes.
When the GUI also has a weekly plan, that plan and the stage condition are combined with `And`.

The example below is a fight task with several stage choices.

```toml
[[tasks]]
name = "理智作战"
type = "Fight"

[[tasks.variants]]
condition = { type = "DateTime", start = "2026-08-08T16:00:00", end = "2026-08-22T03:59:59", timezone = 8 }
params = { stage = "TO-5" }

[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat", "Sun"], timezone = "Official" }
params = { stage = "CE-6" }

[[tasks.variants]]
condition = { type = "Always" }
params = { stage = "LS-6" }
```

If the result will spend Originite Prime, the command warns.
Medicine, Originite Prime, times, and expiring medicine use names that are not in the configuration docs. Those names are in the next section.

### Base rotation

Facility order is `facility`.
Whether the dorm fills operators whose trust is not full is `dorm_trust_enabled`.
Both names are in the infrast example in the configuration docs.

When the GUI rotates shifts by time of day, and every shift in the schedule has a period, `mode` is `10000`.
Variants pick `plan_index` by time of day.
A shift that crosses midnight also gets `DayMod`.
That is the same arrangement as the infrast example in the configuration docs.
When the schedule has no usable period, or the GUI selects a fixed shift, the result stays a custom base plan and includes `plan_index`.

The written `filename` is absolute.
`~` in the schedule path expands to your home directory.
A relative path is resolved from the directory that contains the JSON you passed to the command.
The configuration docs say MaaCore reads that schedule JSON, not maa-cli.
Fiammetta inside the schedule file stays in that file. It is not copied into the task parameters.

To use a different schedule, pass its JSON path with `--custom-schedule`.
If the GUI profile is not already a custom schedule, the command exits with an error.
If that file cannot be read, the command also exits with an error. It does not fall back to the default base plan.

Without that option, a missing file, a file that cannot be parsed, or a file with no shifts writes `mode` as `0`.
The summary says the schedule was skipped.

Drone use, the mood threshold, and the Fiammetta list use names that are not in the configuration docs. Those names are in the next section.

## Parameters passed to MaaCore

The [basic structure](config.md#basic-structure) says the concrete names inside task `params` follow the MAA integration document.
maa-cli does not check those names.
MaaCore reads them when the task runs.
The names below are written into `params` and are not explained in the configuration docs.
Edit these names when you change a value.
Allowed values for each name are in the [MAA integration document][task-types].

### Whether the task runs

When `enable` is `false`, the task does not run.
A task that is off in the GUI is still written, with `enable = false`.
To run it, set that field to `true` or delete it.

### Fight parameters

- `medicine` is how many sanity potions to use. The configuration docs use this name only in a user-input example, and do not say that it is sanity medicine.
- `stone` is how many Originite Prime to use.
- `times` is the battle limit.
- `series` is the number of battles in a row.
- `drops` is the target drop.
- `medicine_expire_days` is how many days of expiring sanity medicine to use. The configuration docs show a different name, `expiring_medicine`. Migration does not write that name.
- When `DrGrandet` is `true`, the task waits on the Originite confirmation until 1 sanity is restored, then spends the Originite.

### Recruitment

- `times` is how many times to recruit.
- `refresh` is whether to refresh a 3-star tag set.
- `extra_tags_mode` is how extra tags are handled. The meaning of the number is in the integration document.
- `select` and `confirm` are the star levels to select and confirm, such as `3`, `4`, and `5`.
- `recruitment_time` is the recruitment minutes for each star level.
- `first_tags` is the preferred tags.
- `preserve_tags` is the tags to keep.
- `level3_recruitment_permit_reserve` is how many 3-star recruitment permits to keep. The name is omitted when that switch is off or the count is not greater than 0.

Immediate refresh from the GUI is not written.
If it is on, the summary has one skipped field and a warning.
If automatic confirmation of a 6-star operator is on, the command warns.

### Base rotation parameters

- `drones` is what drones are used for.
- `threshold` is the dorm mood threshold, from 0 to 1. A percentage in the GUI becomes this fraction.
- `replenish` is whether the trading post restocks Originium Shard.
- `dorm_notstationed_enabled` is whether the dorm uses the "not stationed" option.
- `reception_message_board`, `reception_clue_exchange`, and `reception_send_clue` are the reception room message board, clue exchange, and sending clues.
- `continue_training` is whether the training room keeps training.
- `fiammetta_targets` is the operators Fiammetta should restore mood for.
- `fiammetta_recovery_enabled` is whether to use Fiammetta to restore mood.
- `use_pinus_sylvestris`, `use_perception_information`, `use_worldly_plight`, and `use_abyssal_hunter` are the Pinus Sylvestris, Perception Information, Worldly Plight, and Abyssal Hunter facility sets. A value of `false` is not written.

`fiammetta_targets` applies only when `mode` is 0 and `fiammetta_recovery_enabled` is true.
`fiammetta_recovery_enabled` and those four facility sets apply only when `mode` is 0.

### Credit store

- `shopping` is whether to buy from the credit store.
- `credit_fight` is whether to run the credit fight.
- `formation_index` is the squad used for the credit fight.
- `visit_friends` is whether to visit friends.
- `buy_first` is the items to buy first.
- `blacklist` is the items not to buy.
- `force_shopping_if_credit_full` is whether to buy items outside the blacklist when credit is full.
- `only_buy_discount` is whether to buy only discounted items.
- `reserve_max_credit` is whether to keep credit near the cap.

Last-run times, and whether the credit fight or a friend visit can still be done today, are records kept by the GUI.
They are not written.

### Awards

- `award` collects daily or weekly mission rewards.
- `mail` collects mail rewards.
- `recruit` collects the daily free pull from a limited banner.
- `orundum` collects Orundum from the lucky wall.
- `mining` collects Orundum from a limited mining permit.
- `specialaccess` collects the monthly card given for the 5th anniversary.
- `signinevent` collects a limited-time sign-in reward.

### Roguelike

These settings have not been verified on a real device.
What each parameter does is in the [MAA integration document][task-types].

These names matter only for the matching theme and mode.
They are written so a choice you made in the GUI is not dropped.
MaaCore ignores them in a mode that does not use them.

- `theme` is the theme.
- `mode` is the mode number. Farming score is `0`. Farming Originite Ingot is `1`. Fishing for a start reward is `4`. Collapsal paradigms are `5`. Monthly squad is `6`. Deep exploration is `7`. Finding a Playtime node is `20001`. A BlackFlow start run is `30003`. An unrecognized mode omits `mode` and is recorded in the summary.
- `squad` is the starting squad. `roles` is the starting role set.
- `core_char` is the first starting operator. `core_char_list` is every starting operator. Each item has `name` and `use_support`.
- `use_support` is whether the first operator is a support operator. `use_nonfriend_support` is whether a non-friend support operator is allowed.
- `starts_count` is how many runs to start. `difficulty` is the difficulty. A placeholder that is a very large number is not written.
- `investment_enabled` is whether to invest. `investments_count` is how many investments to make. `investment_with_more_score` is whether to invest for a higher score. `stop_when_investment_full` is whether to stop when the deposit is full.
- `stop_at_final_boss` is whether to stop before the ending. `stop_at_max_level` is whether to stop at max level.
- `start_with_elite_two` and `only_start_with_elite_two` are elite-2 starts.
- `refresh_trader_with_dice` is whether to refresh the trader with dice.
- `collectible_mode_squad` is the squad used while fishing for a start, or for the preparation run in BlackFlow. `collectible_mode_shopping` is whether that run shops. `collectible_mode_start_list` is the start reward to look for, such as `hot_water`.
- `monthly_squad_auto_iterate` is whether the monthly squad switches automatically. `monthly_squad_check_comms` is whether that switch also looks at communications.
- `deep_exploration_auto_iterate` is whether deep exploration switches automatically.
- `find_playTime_target` is the Playtime node. `1` is Ling, `2` is Shu, and `3` is Nian.
- `blackflow_strategy` is the BlackFlow ending strategy, such as `ending1`. `blackflow_cultivation_target` is the swaddled animal to raise. `blackflow_start_animals` is the swaddled animals the preparation run must collect.
- `expected_collapsal_paradigms` is the collapsal paradigms to look for.
- `first_floor_foldartal` and `start_foldartal_list` are foldartal.
- `start_with_seed` is a fixed seed.

The BlackFlow start-reward bitmask and fast flight are not written.
If either one is on in the GUI, and it is not left unset, the summary has a skipped field for it.

### Reclamation

These settings have not been verified on a real device.
What each parameter does is in the [MAA integration document][task-types].

- `theme` is the theme.
- `mode` `0` is prosperity without saving. `mode` `1` is prosperity written into the save. An unrecognized mode is omitted and recorded in the summary.
- `tools_to_craft` is the tools to craft.
- `increment_mode` is the increment mode.
- `num_craft_batches` is how many craft batches to run each round.

Depot recognition and the operator box are not written as tasks.
An unknown task is not written either.
Both appear only in the summary.

## Summary

When something is skipped or turned off, the summary goes to stderr.
When nothing was lost, the command prints nothing.
The text is English.
The three groups below do not all have to appear.

```text
Migration summary (lossy):
  Skipped tasks (unsupported type):
    - UserDataUpdateTask "同步数据"
  Disabled tasks (kept with params.enable=false; will not run):
    - FightTask "剿灭"
  Skipped fields (not migrated):
    - RecruitTask "公开招募": ForceRefresh
```

The first line means this conversion dropped something.

`Skipped tasks` are tasks that were not written at all.
The text in quotes is the task name from the GUI.
`UserDataUpdateTask` is depot recognition and the operator box.

`Disabled tasks` were written, but they do not run.
`params.enable=false` in the parentheses is the `enable` field from the previous section.
The text in quotes is the task name.
`FightTask` is a fight, `RecruitTask` is recruitment, and `InfrastTask` is base rotation.

`Skipped fields` is one setting inside a task that was not written.
The text in quotes is the task name.
The word after the colon is the original field name from the exported file.
You do not need to know that name.
It only means that setting was dropped.
The last line above is what you see when recruitment has immediate refresh turned on.

Read the summary before you use the output for daily runs.

## See also

- [Configuration](config.md)
- [Usage](usage.md)
- [Task types in the MAA integration document][task-types]

[task-types]: https://maa.plus/docs/en-us/protocol/integration.html

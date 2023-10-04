# maa-cli

![CI](https://img.shields.io/github/actions/workflow/status/MaaAssistantArknights/maa-cli/ci.yml)
![maa-cli latest release](https://img.shields.io/github/v/release/MaaAssistantArknights/maa-cli?filter=v*)

A simple CLI for [MAA](https://github.com/MaaAssistantArknights/MaaAssistantArknights) by Rust.
A alternative way use MAA on **Linux** and **macOS**.
Windows is not supported now,
because I don't have a Windows machine
and I'm not familiar with Windows development. PR is welcome.

## Feature

- Install and update MAA core and resources with `maa install` and `maa update`;
- Install and update self with `maa self install` and `maa self update`;
- Define tasks by TOML, YAML or JSON file, then run it by `maa run <task>`, see below for more details;
- Handle MAA core message for monitoring of MAA running status.

## Installation

You can install CLI by download prebuilt binary from
[release page](https://github.com/wangl-cc/maa-cli/releases/latest)
(universal-apple-darwin is for macOS, x86_64-unknown-linux-gnu is for Linux),
and extract it to a directory in your `$PATH` (e.g. `$HOME/.local/bin`).

Once the CLI is installed, you can install `MaaCore` by `maa`:
```bash
maa install
```

**Note**: `adb` is not included in this CLI. Please make sure `adb` is installed, if you connect game with it.

## Usage and configuration

### Run a task

The `maa` is used to run some you defined tasks
(how to define a task will be introduced later):
```sh
maa run <task> [options]
```
More details about `maa run` can be found by `maa run --help`.
And Other commands can be found by `maa --help`.

### Config dir

Your config files (maa options, tasks, etc.) are located in your config dir.
You can get the config dir by `maa dir config` and
create it by `mkdir -p "$(maa dir config)"`.
**Note**: the double quotes is necessary for macOS user with zsh and bash.
Because the path may contains space and will be split into multiple arguments.

*Tip*: For macOS user who prefer to XDG style config directory,
you can set `XDG_CONFIG_HOME`, e.g. `export XDG_CONFIG_HOME="$HOME/.config"`.
Alternatively, you can make a symlink from XDG style dir to Apple style dir:
```sh
mkdir -p "$HOME/.config/maa"
ln -s "$HOME/.config/maa" "$(maa dir config)"
```

In below examples, we assume the config dir is `$MAA_CONFIG_DIR`.

### Maa options

The maa options is a TOML, YAML or JSON file that contains the options of maa,
The maa options contains three sections `connection`, `instance_options` and `resources`.

The `connection` section is used to connect to the game,
the `type` field can be `ADB` or `PlayTools`.
If you use `ADB`, you should set `adb_path` and `device` fields:
```toml
[connection]
type = "ADB"
adb_path = "adb" # the path of adb executable
device = "emulator-5554" # the serial of your android device
config = "General" # the config of maa
```
and if you use `PlayTools`, you should set `address`
which is the address of MaaTools set in PlayCover,
more details can be found at
[here](https://maa.plus/docs/en-us/1.4-EMULATOR_SUPPORTS_FOR_MAC.html#✅-playcover-the-software-runs-most-fluently-for-its-nativity-🚀):
```toml
[connection]
type = "PlayTools"
address = "localhost:1717" # the address of MaaTools
config = "CompatMac" # the same as above
```
Both `ADB` and `PlayTools` can set `config` field,
which is a parameter of `connect` function of maa.
It's default value is `CompatMac` on macOS, `General` on other platforms.
All available values can be found at `resource/config.json` in MAA repo.

And the `instance_options` section is used to configure maa instance options:
```toml
[instance_options]
touch_mode = "ADB" # touch mode to use, can be "ADB", "MiniTouch", "MAATouch" or "MacPlayTools" (only for PlayCover)
deployment_with_pause = false # whether pause the game when deployment
adb_lite_enabled = false # whether use adb-lite
kill_adb_on_exit = false # whether kill adb when exit
```
Note: If you connect to the game with `PlayCover`, the `touch_mode` must be `MacPlayTools`.

The `resources` section is used to configure additional resources of maa,
which is a list of resource directories (relative to `resource` directory of MAA repo):
```toml
resources = ["platform_diff/macOS"]
```
This is useful for adding other server game resources and
platform specific resource.

### Define tasks

A task should be defined with a TOML or JSON file, the located in `$MAA_CONFIG_DIR/tasks`.

#### Basic structure

A task is consists of multiple subtasks,
available subtasks and params are defined by `type` and `params` fields,
it will passed to MaaCore, see [here](https://maa.plus/docs/en-us/3.1-INTEGRATION.html#asstappendtask) for more details:
```toml
[[tasks]]
type = "StartUp" # the type of maa task
params = { client_type = "Official", start_game_enabled = true } # the params of given task
```

#### Task variants and conditions

If you want to run a task with different params based on some conditions,
you can define multiple variants of a task:
```toml
[[tasks]]
type = "Infrast"

[tasks.params]
mode = 10000
facility = ["Trade", "Reception", "Mfg", "Control", "Power", "Office", "Dorm"]
dorm_trust_enabled = true
filename = "normal.json" # the filename of custom infrast plan should located in `$MAA_CONFIG_DIR/infrast`

# use plan 1 before 12:00:00, use plan 2 between 12:00:00 and 18:00:00, use plan 0 after 18:00:00
[[tasks.variants]]
condition = { type = "Time", end = "12:00:00" } # if start is not defined, it will be 00:00:00
params = { plan_index = 1 }

[[tasks.variants]]
condition = { type = "Time", start = "12:00:00", end = "18:00:00" }
params = { plan_index = 2 }

[[tasks.variants]]
condition = { type = "Time", start = "18:00:00" } # if end is not defined, it will be 23:59:59
params = { plan_index = 0 }
```
The `condition` field is used to determine whether the variant should be used,
and the `params` field of matched variant will be merged into the params of the task.

**Note**: this CLI will not read any content inside the infrast plan file,
including the time period defined in the `infrast` file,
so you must define the time period in the `condition` field.

Besides of `Time` condition, there are also `DateTime` and `Weakday` conditions:
```toml
[[tasks]]
type = "Fight"

# fight SL-8 on summer event
[[tasks.variants]]
params = { stage = "SL-8" }
condition = { type = "DateTime", start = "2023-08-01T16:00:00", end = "2023-08-21T03:59:59" }
# fight CE-6 on Tue, Thu, Sat if not on summer event
[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat"] }
params = { stage = "CE-6" }
# fight 1-7 otherwise
[[tasks.variants]]
params = { stage = "1-7" }
```
If multiple variants are matched, the first one will be used.
And if the condition is not given, the variant will always be matched,
So you can put a variant without condition at the end of variants.

If no variant is matched, the task will not be executed,
which is useful when you want to only run a task in some conditions:
```toml
# Mall after 18:00
[[tasks]]
type = "Mall"
[tasks.params]
shopping = true
credit_fight = true
buy_first = ["招聘许可", "龙门币"]
blacklist = ["碳", "家具", "加急许可"]
[[tasks.variants]]
condition = { type = "Time", start = "18:00:00" }
```

#### User input

In some case, you may want to input some value at runtime, instead of hard code it in the task file.
Such as the stage to fight, the item to buy, etc.
You can specify the value `Input` or `Select` type:

```toml
[[tasks]]
type = "Fight"

# Select a stage to fight
[[tasks.variants]]
condition = { type = "DateTime", start = "2023-08-01T16:00:00", end = "2023-08-21T03:59:59" }
[tasks.variants.params.stage]
alternatives = ["SL-6", "SL-7", "SL-8"] # the alternatives of stage, at least one alternative should be given
description = "a stage to fight in summer event" # description of the input, optional

# Task without input
[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat"] }
params = { stage = "CE-6" }

# Input a stage to fight
[tasks.variants]]
[tasks.variants.params.stage]
default = "1-7" # default value of stage, optional (if not given, user can input empty value to re-prompt)
description = "a stage to fight" # description of the input, optional
```

For `Input` type, a prompt will be shown to ask user to input a value.
If the default value is given, it will be used if user input empty value, otherwise it will re-prompt.
For `Select` type, a prompt will be shown to ask user to select a value from alternatives (by index).
If user input is not a valid index, it will re-prompt.


Example of config file can be found at [`config_examples` directory](./config_examples).
Anothor example can be found at my [dotfiles](https://github.com/wangl-cc/dotfiles/tree/master/.config/maa).

### Handle MAA core message

This CLI can handle MAA core message when running a task,
but not all messages will be printed,
the log level is used to control which message will be printed.
There are 6 log level:
- Error: something wrong, the program may exit or not work as expected;
- Warning: something wrong, but the program can still work;
- Normal: some important information, e.g. a task started and finished;
- Info: more detailed information, e.g. stage drop info;
- Debug: details about your configuration, e.g. the params of a task;
  this is useful for you to debug your configuration;
- Trace: any maa message from MAA core which is not handled by this CLI;
  this is useful for developers to debug this CLI.

The default log level is `Normal`, and it can be controlled by `-v` and `-q` options:
`-v` will increase the log level, and `-q` will decrease the log level.

# maa-cli

[中文文档](./README-CN.md)

A simple CLI for [MaaAssistantArknights](https://github.com/MaaAssistantArknights/MaaAssistantArknights) by Rust..
A alternative way use MAA on Linux (and other platform, windows not tested yet).

## Feature

- Define MAA tasks by TOML and JSON file, and run it by `maa run <task>`;
- Callback based on GUI implementation (no complete yet) for better monitoring of MAA running status.

## Installation

This is a CLI tool written in Rust, so you must [install rust](https://www.rust-lang.org/tools/install)
and make sure `cargo` is available.

### Install `MaaCore` and resources

The shared library `MaaCore` is required to build `maa-sys`.
So you must install `MaccCore` at current way before install this CLI.
The easiest way should be `maa-updater`, however it not implemented yet.
So you must do it yourself now.

For macOS user, if you have installed `MAA.app` at `/Applications/MAA.app`,
the libraries can be found at `/Applications/Maa.app/Contents/Frameworks`
and the resources can be found at `/Applications/Maa.app/Contents/Resources/resource`.
Then you can link them to the correct location:
```bash
if [ -n "$XDG_DATA_HOME" ]; then # maa-cli respect XDG Base Directory Specification
    MAA_DATA_DIR="$XDG_DATA_HOME/maa"
else
    MAA_DATA_DIR="$HOME/Library/Application\ Support/com.loong.maa"
fi
ln -s /Applications/Maa.app/Contents/Frameworks "$MAA_DATA_DIR/lib"
ln -s /Applications/Maa.app/Contents/Resources/resource "$MAA_DATA_DIR/resource"
```

For Linux user, you should download the latest release of `MAA` from [here](https://maa.plus).
Then, if you have Downloaded the `MAA` at `~/Downloads`, then you can extract it to the correct location:
```bash
MAA_DOWNLOAD_DIR="$HOME/Downloads"
if [ -n "$XDG_DATA_HOME" ]; then # maa-cli respect XDG Base Directory Specification
    MAA_DATA_DIR="$XDG_DATA_HOME/maa"
else
    MAA_DATA_DIR="$HOME/.local/share/maa"
fi
mkdir -p "$MAA_DATA_DIR"
tar -xzf $MAA_DOWNLOAD_DIR/MAA*.tar.gz -C "$MAA_DATA_DIR"
mkdir "$MAA_DATA_DIR/lib"
mv $MAA_DATA_DIR/lib*.so* "$MAA_DATA_DIR/lib"
rm -r "$MAA_DATA_DIR/Python" $MAA_DATA_DIR/*.h
```

For windows user, I have no idea how to do it now, because I don't have a windows machine.
If you want to use it on windows, try install `maa-updater`:
```bash
cargo install --git https://github.com/wangl-cc/maa-cli maa-updater --locked
```
and use `maa-updater package` to get correct location of libraries and resources.

### Install `maa-cli`

Once the maa core is installed at correct location, you can install `maa-cli`:
```sh
cargo install --git https://github.com/wangl-cc/maa-cli maa-cli --locked
```

## Usage

The `maa-cli` is used to run some you defined tasks (how to define a task will be introduced later):
```sh
maa run <task> [options]
```
More details can be found at `maa --help`.

### Config dir

Your config files (maa options, tasks, etc.) are located in your config dir,
see [directories-rs](https://crates.io/crates/directories) for more details.
which is `$HOME/.config/maa` on Linux and `$HOME/Library/Application Support/com.loong.maa/config` on macOS by default.
The path can be changeed by set environment variable `MAA_CONFIG_DIR`,
or set `XDG_CONFIG_HOME` (the config dir of maa will be `$XDG_CONFIG_HOME/maa`).

In below examples, we assume the config dir is `$MAA_CONFIG_DIR`.

### Maa options

The maa options should be defined with a TOML or JSON file,
the located in `$MAA_CONFIG_DIR/asst.toml` or `$MAA_CONFIG_DIR/asst.json`.
The maa options contains two sections: `connection` and `instance_options`.

The `connection` section is used to connect to the game:
```toml
[connection]
type = "ADB" # or "PlayCover" which is not implemented yet
adb_path = "adb" # the path of adb executable
device = "emulator-5554" # the serial of your android device
config = "General" # the config of maa connect, default to `CompatMac` on macOS, `General` on other platforms, see resource/config.json in MAA repo for more details
```

And the `instance_options` section is used to configure maa instance options:
```toml
[instance_options]
touch_mode = "ADB" # touch mode to use, can be "ADB", "MiniTouch", "MaaTouch"  or "MacPlayTools"(not works now)
deployment_with_pause = false # whether pause the game when deployment
adb_lite_enabled = false # whether use adb-lite
kill_adb_on_exit = false # whether kill adb when exit
```

### Define tasks

A task should be defined with a TOML or JSON file, the located in `$MAA_CONFIG_DIR/tasks`.

A task is consists of multiple subtasks,
each subtask is a [MAA task china](https://maa.plus/docs/3.1-集成文档.html#asstappendtask):
```toml
[[tasks]]
type = "StartUp" # the type of maa task
params = { client_type = "Official", start_game_enabled = true } # the params of given task
```

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

A complete example please see my [dotfiles](https://github.com/wangl-cc/dotfiles/tree/master/.config/maa).

## TODO

- [ ] maa-updater
- [ ] Better message processing
  - [ ] Rouge-like mode message processing
  - [ ] Subtask extra info processing
- [ ] PlayCover support

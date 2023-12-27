# maa-cli

![CI](https://img.shields.io/github/actions/workflow/status/MaaAssistantArknights/maa-cli/ci.yml)
![Code coverage](https://img.shields.io/codecov/c/github/MaaAssistantArknights/maa-cli)
![Stable Release](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fgithub.com%2FMaaAssistantArknights%2Fmaa-cli%2Fraw%2Fversion%2Fstable.json&query=%24.version&prefix=v&label=stable)
![Beta Release](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fgithub.com%2FMaaAssistantArknights%2Fmaa-cli%2Fraw%2Fversion%2Fbeta.json&query=%24.version&prefix=v&label=beta)
![platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blueviolet)

A simple CLI for [MAA](https://github.com/MaaAssistantArknights/MaaAssistantArknights) by Rust.

## Feature

- Run predefined or custom tasks, like `maa fight` or `maa run <task>`;
- Install and update `MaaCore` and resources with `maa install` and `maa update`;
- Update self with `maa self update`.

## Installation

### Package manager

#### macOS

Install with [Homebrew](https://brew.sh/):

```bash
brew install MaaAssistantArknights/tap/maa-cli
```

#### Linux

Arch Linux user can install [AUR package](https://aur.archlinux.org/packages/maa-cli/):

```bash
yay -S maa-cli
```

For Linux Brew user, you can install with [Linux Brew](https://docs.brew.sh/Homebrew-on-Linux):

```bash
brew install MaaAssistantArknights/tap/maa-cli
```

### Prebuilt binary

You can install CLI by download prebuilt binary from
[release page](https://github.com/wangl-cc/maa-cli/releases/latest) and extract it to your favourite location. The filename for different platform is:

<table>
    <thead>
        <tr>
            <th>Operation System</th>
            <th>Architecture</th>
            <th>Filename</th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td rowspan=2>Linux</td>
            <td>x86_64</td>
            <td>maa_cli-x86_64-unknown-linux-gnu.tar.gz</td>
        </tr>
        <tr>
            <td>aarch64</td>
            <td>maa_cli-aarch64-unknown-linux-gnu.tar.gz</td>
        </tr>
        <tr>
            <td rowspan=2>macOS</td>
            <td>x86_64</td>
            <td rowspan=2>
              maa_cli-universal-apple-darwin.zip
            </td>
        </tr>
        <tr>
            <td>aaarch64</td>
        </tr>
        <tr>
            <td rowspan=2>Windows</td>
            <td>x86_64</td>
            <td>maa_cli-x86_64-pc-windows-msvc.zip</td>
        </tr>
    </tbody>
</table>

### Build from source

You can also build from source by yourself with `cargo`:

```bash
cargo install --git https://github.com/MaaAssistantArknights/maa-cli.git --bin maa --locked
```

#### Build options

When building from source, you can disable default features with `--no-default-features` option and enable specific features with `--features` option. Currently, the available features are:

- `cli_installer`: Provide `maa self update` command to update self, this feature is enabled by default;
- `core_installer`: Provide `maa install` and `maa update` commands to install and update `MaaCore` and resources, this feature is enabled by default;
- `git2`: Provide `libgit2` resource backend, this feature is enabled by default;
- `vendored-openssl`: Build `openssl` library by self instead of using system `openssl` library, this feature is disabled by default;

### Dependencies

#### MaaCore

`maa-cli` only provides an interface for MaaCore, it needs `MaaCore` and resources to run tasks, which can be installed by `maa install`:

```bash
maa install
```

#### OpenSSL

`git2` depends on the `openssl` library on all platforms. On Linux, it is also required by `maa-cli` itself. So you should install the `openssl` library or use the `vendored-openssl` feature when building from source.

## Usage

### Run Tasks

The main feature of `maa-cli` is to run tasks, including predefined tasks and custom tasks.

#### Predefined tasks

- `maa fight [stage]`: run a fight task, the `stage` is the stage to fight, like `1-7`, `CE-6`, etc; if not given, it will be queried from user;
- `maa copilot <maa_uri>`: run a copilot task, the `maa_uri` is the URI of a copilot task; it can be `maa://1234` or local file path;

#### Custom tasks

You can run a custom task by `maa run <task>`. Here `<task>` is the name of a task, you can list all available tasks by `maa list`.

#### Task Summary

`maa-cli` will print a summary of each task to stdout when finished. The summary can be disabled by `--no-summary` option.

### Install and update

#### Install and update for MaaCore and resources

You can install and update `MaaCore` and resources by `maa install` and `maa update`. See `maa help install` and `maa help update` for more details.

#### Resource hot update

You can hot update resources by `maa  hot-update`. It can be configured to run every time before running in config file.

#### Self update

You can update `maa-cli` by `maa self update`. For users who install `maa-cli` with package manager, this feature is disabled, you should update `maa-cli` with package manager.

More other commands can be found by `maa help`.

### Other subcommands

- `maa list`: list all available tasks;
- `maa dir <subcommand>`: get the path of a specific directory;
- `maa version`: print the version of `maa-cli` and `MaaCore`;
- `maa convert <input> [output]`: convert a configuration file to another format, like `maa convert daily.toml daily.json`;
- `maa complete <shell>`: generate completion script for specific shell;

## Configurations

### Configuration directory

All configurations of `maa-cli` are located in a specific configuration directory, which can be got by `maa dir config`.
The configuration directory can be changed by environment variable `MAA_CONFIG_DIR`. In below examples, we will use `$MAA_CONFIG_DIR` to represent the configuration directory.

All configuration files can be written in TOML, YAML or JSON format. In below examples, we will use TOML format and `.toml` as file extension. But you can mix these three formats as long as the file extension is correct.

<details>

<summary> XDG style configuration directory on macOS </summary>

Due to the limitation of [Directories](https://github.com/dirs-dev/directories-rs/), `maa-cli` use Apple style configuration directory on macOS by default. But XDG style configuration directory is more suitable for command line program. If you want to use XDG style configuration directory, you can set `XDG_CONFIG_HOME` environment variable, such as `export XDG_CONFIG_HOME="$HOME/.config"`, this will make `maa-cli` use XDG style configuration directory. Or you can use below command to create a symbolic link:

```bash
mkdir -p "$HOME/.config/maa"
ln -s "$HOME/.config/maa" "$(maa dir config)"
```

</details>

### Define tasks

A `maa-cli` task should be defined in a single file, which should be located in `$MAA_CONFIG_DIR/tasks` directory.

#### Basic structure

A `maa-cli` task is a sequence of `MAA` tasks, each `MAA` task is defined by `name`, `type` and `params` fields:

```toml
[[tasks]]
name = "Start Game" # the name this task, default to the type of the task
type = "StartUp" # the type of maa task
params = { client_type = "Official", start_game_enabled = true } # the params of given task
```

See documentation of [MAA](https://maa.plus/docs/en-us/3.1-INTEGRATION.html#asstappendtask) for all available task types and parameters.

#### Task variants and conditions

In some cases, you may want to run a task with different parameters in different conditions. You can define multiple variants for a task, and use the `condition` field to determine whether the variant should be used. For example, you may want to use a different infrastructure plan in different time periods of a day:

```toml
[[tasks]]
type = "Infrast"

[tasks.params]
mode = 10000
facility = ["Trade", "Reception", "Mfg", "Control", "Power", "Office", "Dorm"]
dorm_trust_enabled = true
filename = "normal.json" # the filename of custom infrast plan

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
and the `params` field of matched variant will be merged into the parameters of the task.

**Note**: If the `filename` field is a relative path, it will be relative to `$MAA_CONFIG_DIR/infrast`. Besides, the custom infrastructure plan file will not be read by `maa-cli` but `MaaCore`. So the format of the file must be `JSON` and time period defined in the file will not be used to select the corresponding sub-plan. So you must specify the `plan_index` field in the parameters of the task to use the correct infrastructure plan in the corresponding time period. This will ensure that the correct infrastructure plan is used in the appropriate time period.

Besides of `Time` condition, there are also `DateTime`, `Weakday`, and `Combined` conditions. `DateTime` condition is used to specify a specific datetime period, `Weekday` condition is used to specify some days in a week, `Combined` condition is used to specify a combination of multiple conditions.

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

With default strategy, if multiple variants are matched, only the first one will be used. And if the condition is not given, the variant will always be matched. So you can put a variant without condition at the end of variants.

The strategy of matching variants can be changed by `strategy` field:

```toml
[[tasks]]
type = "Fight"
strategy = "merge" # or "first" (default)

# use all expiring medicine on Sunday night
[[tasks.variants]]
params = { expiring_medicine = 1000 }
[tasks.variants.condition]
type = "Combined"
conditions = [
  { type = "Time", start = "18:00:00" },
  { type = "Weekday", weekdays = ["Sun"] },
]

# fight 1-7 by default
[[tasks.variants]]
params = { stage = "1-7" }

# fight CE-6 on Tue, Thu, Sat if not on summer event
[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat"] }
params = { stage = "CE-6" }

# fight SL-8 on summer event
[[tasks.variants]]
params = { stage = "SL-8" }
condition = { type = "DateTime", start = "2023-08-01T16:00:00", end = "2023-08-21T03:59:59" }
```

The outcome stage of this example should be identical to the previous one, but expiring medicine will be used on Sunday night additionally.
With the `merge` strategy, if multiple variants are matched, the parameters of all matched variants will be merged. If multiple variants have the same parameters, the last one will be used.

If no variant is matched, the task will not be executed,
which is useful when you want to only run a task in some conditions:

```toml
# Mall after 18:00
[[tasks]]
type = "Mall"

[[tasks.variants]]
condition = { type = "Time", start = "18:00:00" }
```

#### User input

In some case, you may want to input some value at runtime, instead of hard code it in the task file. Such as the stage to fight, the item to buy, etc. You can specify the value as `Input` or `Select` type:

```toml
[[tasks]]
type = "Fight"

# Select a stage to fight
[[tasks.variants]]
condition = { type = "DateTime", start = "2023-08-01T16:00:00", end = "2023-08-21T03:59:59" }

# Set the stage to a `Select` type with alternatives and description
[tasks.variants.params.stage]
alternatives = ["SL-6", "SL-7", "SL-8"] # the alternatives of stage, at least one alternative should be given
description = "a stage to fight in summer event" # description of the input, optional

# Task without input
[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat"] }
params = { stage = "CE-6" }

# Input a stage to fight
[[tasks.variants]]

# Set the stage to a `Input` type with default value and description
[tasks.variants.params.stage]
default = "1-7" # default value of stage, optional (if not given, user can input empty value to re-prompt)
description = "a stage to fight" # description of the input, optional
```

For `Input` type, a prompt will be shown to ask user to input a value. If the default value is given, it will be used if user input empty value, otherwise it will re-prompt. For `Select` type, a prompt will be shown to ask user to select a value from alternatives (by index). If user input is not a valid index, it will re-prompt. To promote and input can be disabled by `--batch` option, which is useful for running tasks in Schedule.

Example of config file can be found at [`config_examples` directory](./maa-cli/config_examples). Another example can be found at my [dotfiles](https://github.com/wangl-cc/dotfiles/tree/master/.config/maa).

### `MaaCore` related configurations

The related configurations of `MaaCore` is located in `$MAA_CONFIG_DIR/asst.toml`. The current available configurations are:

```toml
[connection]
type = "ADB"
adb_path = "adb"
device = "emulator-5554"
config = "CompatMac"

[resource]
global_resource = "YoStarEN"
platform_diff_resource = "iOS"
user_resource = true

[static_options]
cpu_ocr = false
gpu_ocr = 1

[instance_options]
touch_mode = "MAATouch"
deployment_with_pause = false
adb_lite_enabled = false
kill_adb_on_exit = false
```

#### Connection

The `connection` section is used to specify how to connect to the game. Currently, there are two types of connection: `ADB` and `PlayTools`.

If you use `ADB`, you should set `adb_path` and `device` fields:

```toml
[connection]
type = "ADB"
adb_path = "adb" # the path of adb executable
device = "emulator-5554" # the serial of your android device
config = "General" # the config of maa
```

Note, the `device` field is any valid input of `-s` option of `adb` command, like `emulator-5554` or `127.0.0.1:5555`.

If you use `PlayTools`, you should set `address`
which is the address of `MaaTools` set in `PlayCover`,
more details can be found at
[here](https://maa.plus/docs/en-us/1.4-EMULATOR_SUPPORTS_FOR_MAC.html#✅-playcover-the-software-runs-most-fluently-for-its-nativity-🚀):

```toml
[connection]
type = "PlayTools"
address = "localhost:1717" # the address of MaaTools
config = "CompatMac" # the same as above
```

Both `ADB` and `PlayTools` share the `config` field, which is a parameter of `connect` function of MAA. Its default value is `CompatMac` on macOS, `CompatPOSIXShell` on Linux and `General` on other platforms. More optional configs can be found in `config.json` in resource directory.

#### Resource

The `resource` section is used to specify the resource to use:

```toml
[resource]
global_resource = "YoStarEN" # the global resource to use
platform_diff_resource = "iOS" # the platform diff resource to use
user_resource = true # whether use user resource
```

When your game is not in Simplified Chinese, you should set `global_resource` to non-Chinese resource. If you connect to the game with `PlayCover`, you should set `platform_diff_resource` to `iOS`.
Leave those two fields to empty if you don't want to use global resource or platform diff resource. Besides, those two fields will also be setup automatically by `maa-cli` based on your task and connection type.
Lastly, if you want to use user resource, you should set `user_resource` to `true`. When `user_resource` is `true`, `maa-cli` will try to find user resource in `$MAA_CONFIG_DIR/resource` directory.

#### Static options

The `static_options` section is used to configure MAA [static options](https://maa.plus/docs/en-us/3.1-INTEGRATION.html#asstsetstaticoption):

```toml
[static_options]
cpu_ocr = false # whether use CPU OCR, CPU OCR is enabled by default
gpu_ocr = 1 # the ID of your GPU, leave it to empty if you don't want to use GPU OCR
```

#### Instance options

The `instance_options` section is used to configure MAA [instance options](https://maa.plus/docs/en-us/3.1-INTEGRATION.html#asstsetinstanceoption):

```toml
[instance_options]
touch_mode = "ADB" # touch mode to use, can be "ADB", "MiniTouch", "MAATouch" or "MacPlayTools" (only for PlayCover)
deployment_with_pause = false # whether pause the game when deployment
adb_lite_enabled = false # whether use adb-lite
kill_adb_on_exit = false # whether kill adb when exit
```

Note: If you connect to the game with `PlayCover`, the `touch_mode` will be ignored and `MacPlayTools` will be used.

### `maa-cli` related configurations

The `maa-cli` related configurations should be located in `$MAA_CONFIG_DIR/cli.toml`. Currently, it only contains one section: `core`:

```toml
# MaaCore install and update  configurations
[core]
channel = "Stable" # update channel, can be "Stable", "Beta" or "Alpha"
test_time = 0 # the time to test download mirrors in seconds, 0 to skip
# the url to query the latest version of MaaCore, leave it to empty to use default url
apit_url = "https://github.com/MaaAssistantArknights/maa-cli/raw/version/"
[core.components]
library = true # whether install MaaCore library
resource = false # whether install resoruce resource

# CLI update configurations
[cli]
channel = "Stable" # update channel, can be "Stable", "Beta" or "Alpha"
# the url to query the latest version of maa-cli, leave it to empty to use default url
api_url = "https://github.com/MaaAssistantArknights/maa-cli/raw/version/"
# the url to download prebuilt binary, leave it to empty to use default url
download_url = "https://github.com/MaaAssistantArknights/maa-cli/releases/download/"

[cli.components]
binary = true # whether install maa-cli binary


# hot update resource configurations
[resource]
auto_update = true # whether auto update resource before running task
backend = "libgit2" # the backend of resource, can be "libgit2" or "git"

# the remote of resource
[resource.remote]
branch = "main" # the branch of remote repository
# the url of remote repository, when using ssh, you should set ssh_key field
url = "https://github.com/MaaAssistantArknights/MaaResource.git"
# url = "git@github.com:MaaAssistantArknights/MaaResource.git"
# ssh_key = "~/.ssh/id_ed25519" # path to ssh key
```

**NOTE**:

- The `Alpha` channel of `MaaCore` is only available on Windows;
- The hot update resource can not work separately, it should be used with basic resource that installed with `MaaCore`;
- If you want to use `git` backend, `git` command is required;
- If you want to fetch resource with ssh, the `ssh_key` is required;
- The `resource.remote.url` only effect for first time installation, it will be ignored when updating resource. If you want to change the remote url, you should change it manually or delete the resource directory and reinstall resource. The directory of repository can be located by `maa dir hot-update`.

### JSON schema

The JSON schema of config file can be found at [`schemas` directory](./maa-cli/schemas/).
The schema of task file is [`task.schema.json`](./maa-cli/schemas/task.schema.json);
the schema of MaaCore config file is [`asst.schema.json`](./maa-cli/schemas/asst.schema.json);
the schema of CLI config file is [`cli.schema.json`](./maa-cli/schemas/cli.schema.json);

With the help of JSON schema, you can get auto completion and validation in some editors with plugins.

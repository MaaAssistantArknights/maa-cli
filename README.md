# maa-cli

![CI](https://img.shields.io/github/actions/workflow/status/MaaAssistantArknights/maa-cli/ci.yml)
![maa-cli latest release](https://img.shields.io/github/v/release/MaaAssistantArknights/maa-cli?filter=v*)
![platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blueviolet">)
![Codecov](https://img.shields.io/codecov/c/github/MaaAssistantArknights/maa-cli)

[English](./README-EN.md)

一个使用rust编写的简单[MAA](https://github.com/MaaAssistantArknights/MaaAssistantArknights)命令行工具。

## 功能

- 通过 TOML，YAML 或者 JSON 文件定义任务，并通过 `maa run <task>` 执行；
- 使用 `maa install` 和 `maa update` 安装和更新`MaaCore`及资源；
- 使用 `maa self update` 更新自身；

## 安装

### 包管理器

#### macOS

使用 [Homebrew](https://brew.sh/) 安装：

```bash
brew install MaaAssistantArknights/tap/maa-cli
```

#### Linux

ArchLinux 用户可以安装[AUR包](https://aur.archlinux.org/packages/maa-cli/):

```bash
yay -S maa-cli
```

对于 LinuxBrew 用户，可以使用 [LinuxBrew](https://docs.brew.sh/Homebrew-on-Linux) 安装：

```bash
brew install MaaAssistantArknights/tap/maa-cli
```

### 预编译二进制文件

你可以从 [release 页面](https://github.com/MaaAssistantArknights/maa-cli/releases/latest)下载预编译的二进制文件，将其解压后得到的可执行文件放在你喜欢的位置。不同的平台对应的文件名如下：

<table>
    <thead>
        <tr>
            <th>操作系统</th>
            <th>处理器架构</th>
            <th>文件名</th>
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

### 从源码编译

你也可以通过 `cargo` 从源码编译安装：

```bash
cargo install --git https://github.com/MaaAssistantArknights/maa-cli.git --bin maa --locked
```

### 依赖

#### MaaCore

`maa-cli` 只提供了一个命令行界面，它需要 `MaaCore` 和资源来运行任务。一旦 `maa-cli`  安装完成，你可以通过 `maa install` 命令安装 `MaaCore` 及资源：

```bash
maa install
```

## 使用

`maa-cli` 的主要功能是运行任务，你可以通过 `maa run <task>` 来运行一个任务。这里的 `<task>` 是一个任务的名字，你可以通过 `maa list` 来列出所有可用的任务。

更多信息可以通过 `maa help` 获取。

## 配置

### 配置目录

`maa-cli` 配置文件位于特定的配置目录中，你可以通过`maa dir config`获取配置目录。配置目录也可以通过环境变量 `MAA_CONFIG_DIR` 更改。在下面的例子中，我们将用 `$MAA_CONFIG_DIR` 来表示配置目录。

所有的配置文件都可以使用 TOML，YAML 或者 JSON 格式，在下面的例子中，我们将使用 TOML 格式，并使用 `.toml` 作为文件扩展名。但是你可以混合这三种格式中的任意一种，只要你的文件扩展名正确。

<details>

<summary> 在 macOS 上使用 XDG 风格配置目录 </summary>

由于 Rust 库 [Directories](https://github.com/dirs-dev/directories-rs/) 在 macOS 上默认使用 Apple 风格目录，`maa-cli` 默认也使用 Apple 风格的配置目录。但是对于命令行程序来说，XDG 风格的目录更加合适。如果你想要使用 XDG 风格目录，你可以设置 `XDG_CONFIG_HOME` 环境变量，如 `export XDG_CONFIG_HOME="$HOME/.config"`，这会让 `maa-cli` 使用 XDG 风格配置目录。如果你想要使用 XDG 风格配置目录，但是不想设置环境变量，你可以使用下面的命令创建一个符号链接：

```bash
mkdir -p "$HOME/.config/maa"
ln -s "$HOME/.config/maa" "$(maa dir config)"
```

</details>

### 定义自定义任务

每一个自定义任务都是一个单独的文件，它们应该位于 `$MAA_CONFIG_DIR/tasks` 目录中。

#### 基本结构

一个任务文件包含多个子任务，每一个子任务是一个[MAA任务](https://maa.plus/docs/3.1-集成文档.html#asstappendtask)：

```toml
[[tasks]]
type = "StartUp" # maa任务的类型
params = { client_type = "Official", start_game_enabled = true } # maa任务的参数
```

#### 任务条件

如果你想要根据一些条件运行不同参数的任务，你可以定义多个任务的变体：

```toml
[[tasks]]
type = "Infrast"

[tasks.params]
mode = 10000
facility = ["Trade", "Reception", "Mfg", "Control", "Power", "Office", "Dorm"]
dorm_trust_enabled = true
filename = "normal.json" # 自定义的基建计划的文件名应该位于`$MAA_CONFIG_DIR/infrast`

# 在12:00:00之前使用计划1，在12:00:00到18:00:00之间使用计划2，在18:00:00之后使用计划0
[[tasks.variants]]
condition = { type = "Time", end = "12:00:00" } # 如果没有定义start，那么它将会是00:00:00
params = { plan_index = 1 }

[[tasks.variants]]
condition = { type = "Time", start = "12:00:00", end = "18:00:00" }
params = { plan_index = 2 }

[[tasks.variants]]
condition = { type = "Time", start = "18:00:00" }
params = { plan_index = 0 }
```

这里的 `condition` 字段用于确定哪一个变体应该被使用，而匹配的变体的 `params` 字段将会被合并到任务的参数中。

**注意**：如果你的自定义基建计划文件使用相对路径，应该相对于 `$MAA_CONFIG_DIR/infrast`。此外，由于基建文件是由 `MaaCore` 而不是 `maa-cli` 读取的，因此这些文件的格式必须是 `JSON`。同时，`maa-cli` 不会读取基建文件，也不会根据其中定义的时间段来选择相应的子计划。因此，必须通过 `condition` 字段来指定在相应时间段使用正确的基建计划的参数中的 `plan_index` 字段。这样可以确保在适当的时间段使用正确的基建计划。

除了 `Time` 条件，还有 `DateTime`，`Weakday` 以及 `Combined` 条件。`DateTime` 条件用于指定一个时间段，`Weekday` 条件用于指定一周中的某些天，`Combined` 条件用于指定多个条件的组合。

```toml
[[tasks]]
type = "Fight"

# 在夏活期间，刷SL-8
[[tasks.variants]]
params = { stage = "SL-8" }
condition = { type = "DateTime", start = "2023-08-01T16:00:00", end = "2023-08-21T03:59:59" }

# 在夏活期间以外的周二、周四和周六，刷CE-6
[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat"] }
params = { stage = "CE-6" }

# 其他时间，刷1-7
[[tasks.variants]]
params = { stage = "1-7" }
```

在默认的策略下，如果有多个变体被匹配，第一个将会被使用。如果没有给出条件，那么变体将会总是被匹配，所以你可以把没有条件的变体放在最后，作为默认的情况。

你可以使用 `strategy` 字段来改变匹配策略：

```toml
[[tasks]]
type = "Fight"
strategy = "merge" # 或者 "first" (默认)

# 在周天晚上使用所有的将要过期的理智药
[[tasks.variants]]
params = { expiring_medicine = 1000 }

[tasks.variants.condition]
type = "Combined"
conditions = [
  { type = "Time", start = "18:00:00" },
  { type = "Weekday", weekdays = ["Sun"] },
]

# 默认刷1-7
[[tasks.variants]]
params = { stage = "1-7" }

# 在周二、周四和周六，刷CE-6
[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat"] }
params = { stage = "CE-6" }

# 在夏活期间，刷SL-8
[[tasks.variants]]
params = { stage = "SL-8" }
condition = { type = "DateTime", start = "2023-08-01T16:00:00", end = "2023-08-21T03:59:59" }
```

这个例子和上面的例子将刷同样的关卡，但是在周天晚上，将会使用所有的将要过期的理智药。在 `merge` 策略下，如果有多个变体被匹配，后面的变体的参数将合并入前面的变体的参数中。如果多个变体都有相同的参数，那么后面的变体的参数将会覆盖前面的变体的参数。

如果没有变体被匹配，那么任务将不会被执行，这可以用于只在特定的条件下运行子任务：

```toml
# 只在在18:00:00之后进行信用商店相关的操作
[[tasks]]
type = "Mall"

[[tasks.variants]]
condition = { type = "Time", start = "18:00:00" }
```

#### 用户输入

对于一些任务，你可能想要在运行时输入一些参数，例如关卡名称。 你可以将对应需要输入的参数设置为 `Input` 或者 `Select` 类型：

```toml
[[tasks]]
type = "Fight"

# 选择一个关卡
[[tasks.variants]]
condition = { type = "DateTime", start = "2023-08-01T16:00:00", end = "2023-08-21T03:59:59" }
[tasks.variants.params.stage]
alternatives = ["SL-6", "SL-7", "SL-8"] # 可选的关卡，必须提供至少一个可选值
description = "a stage to fight in summer event" # 描述，可选

# 无需任何输入
[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat"] }
params = { stage = "CE-6" }

# 输入一个关卡
[[tasks.variants]]
[tasks.variants.params.stage]
default = "1-7" # 默认的关卡，可选（如果没有默认值，输入空值将会重新提示输入）
description = "a stage to fight" # 描述，可选
```

对于 `Input` 类型，当运行任务时，你将会被提示输入一个值。如果你输入了一个空值，那么默认值将会被使用。
对于 `Select` 类型，当运行任务时，你将会被提示选择一个值 （通过输入可选值的序号）。
注意，当你的输入不是可选值时，你将会被提示重新输入。

配置文件的例子可以在[`config_examples`目录](./config_examples)中找到。
另一个例子是我自己的配置文件，你可以在[这里](https://github.com/wangl-cc/dotfiles/tree/master/.config/maa)找到。

### MaaCore 相关配置

和 MaaCore 相关的配置需要放在 `$MAA_CONFIG_DIR/asst.toml` 中。
目前其包含的配置有：

```toml
user_resource = true
resources = ["platform_diff/iOS"]

[connection]
type = "ADB"
adb_path = "adb"
device = "emulator-5554"
config = "CompatMac"

[instance_options]
touch_mode = "MAATouch"
deployment_with_pause = false
adb_lite_enabled = false
kill_adb_on_exit = false
```

`user_resource` 字段用于指定是否从用户资源目录加载资源，它的默认值是`false`。这个参数和命令行选项 `--user-resource` 相同。更多信息可以通过 `maa help run` 获取。

`resources`字段用于指定资源的路径，这是一个资源目录列表，如果是相对路径，它应该相对于 `resource` 目录（`maa dir resource`）。这可以用来加载外服资源或者平台差异资源。

`[connection]` 相关字段用于指定 MaaCore 连接游戏的方式和参数。目前可用的连接方式有 `ADB` 和 `PlayTools`。

当你使用 `ADB` 连接时，你需要提供 `adb` 的路径和设备的序列号：

```toml
[connection]
type = "ADB"
adb_path = "adb" # adb可执行文件的路径
device = "emulator-5554" # 你的android设备的序列号
config = "General" # maa connect的配置
```

当你使用 `PlayTools` 连接时，你需要提供 `PlayTools` 的地址：

```toml
[connection]
type = "PlayCover"
address = "localhost:1717" # PlayTools的地址
config = "CompatMac" # maa connect的配置
```

两者都需要提供 `config`，这个值将被传给 MaaCore，用于指定一些平台和模拟器相关的配置。对于 Linux 他默认为 `CompatPOSIXShell`，对于 macOS 他默认为 `CompatMac`，对于 Windows 他默认为 `General`。更多可选配置可以在资源文件夹中的 `config.json` 文件中找到。

`[instance_options]` 相关字段用于指定 MaaCore 实例的选项，详见 [MAA文档](https://maa.plus/docs/3.1-集成文档.html#asstsetinstanceoption)：

```toml
[instance_options]
touch_mode = "ADB" # 使用的触摸模式，可选值为"ADB", "MiniTouch", "MAATouch"  或者 "MacPlayTools"(仅适用于PlayCover)
deployment_with_pause = false # 是否在部署时暂停游戏
adb_lite_enabled = false # 是否使用adb-lite
kill_adb_on_exit = false # 是否在退出时杀死adb
```

注意，如果你使用`PlayTools`，`touch_mode`字段将被忽略并被设置为`MacPlayTools`。

### `maa-cli` 相关配置

`maa-cli` 相关的配置需要放在 `$MAA_CONFIG_DIR/cli.toml` 中。目前其包含的配置如下：

```toml
[core]
channel = "beta"
components.resource = false
```

`[core]` 相关字段用于指定 CLI 安装和更新 `MaaCore` 的方式。`channel` 字段用于指定 `MaaCore` 的更新通道，可选值为 `stable` 和 `beta`，`alpha`。`components.resource` 字段用于指定是否安装资源。

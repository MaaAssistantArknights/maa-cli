# maa-cli

![CI](https://img.shields.io/github/actions/workflow/status/wangl-cc/maa-cli/ci.yml)
![maa-cli latest release](https://img.shields.io/github/v/release/wangl-cc/maa-cli?label=CLI&filter=maa_cli-*)
![maa-run latest release](https://img.shields.io/github/v/release/wangl-cc/maa-cli?label=Run&filter=maa_run-*)

一个使用rust编写的简单[MAA](https://github.com/MaaAssistantArknights/MaaAssistantArknights)命令行工具。
支持Linux和macOS，Windows暂不支持，因为我没有Windows机器，也不熟悉Windows开发，欢迎PR。

## 功能

- 使用`maa install`和`maa update`安装和更新MaaCore共享库和资源；
- 使用`maa self install`和`maa self update`安装和更新CLI自己；
- 通过TOML，YAML或者JSON文件定义MAA任务，并通过`maa run <task>`执行；
- 处理MAA的消息，用于监控MAA的运行状态。

## 安装

这个CLI由两部分组成：`maa-cli`（提供`maa`命令）和`maa-run`。
但是你只需要安装`maa-cli`就可以使用这个CLI。
你可以从[release页面](https://github.com/wangl-cc/maa-cli/releases/latest),
下载预编译的二进制文件，然后解压到你的`$PATH`中（例如`$HOME/.local/bin`）。

一旦CLI安装完成，你可以通过`maa`安装`maa-run`和`MaaCore`：
```bash
maa install && maa self install
```

**注意**：本工具不包含`adb`，如果使用`adb`来连接游戏，请确保`adb`已正确安装。

## 使用和配置

### 运行任务

`maa`用于运行你定义的任务（如何定义任务稍后介绍）：
```bash
maa run <task> [options]
```
更多关于`maa run`的细节可以通过`maa run -- --help`查看。
`maa`其他可用命令可以通过`maa --help`查看。

### 配置目录

你的配置文件（maa选项，任务等）位于配置目录中。
你可以通过`maa dir config`获取配置目录,
并通过`mkdir -p "$(maa dir config)"`创建它。
**注意**：对于macOS上使用zsh和bash的用户，双引号是必须的,
因为路径可能包含空格，这会导致它被分割成多个参数。

*提示*：对于macOS上喜欢XDG风格配置目录的用户，
你可以设置`XDG_CONFIG_HOME`，例如`export XDG_CONFIG_HOME="$HOME/.config"`。
或者，你可以创建一个从XDG风格目录到Apple风格目录的符号链接：
```sh
mkdir -p "$HOME/.config/maa"
ln -s "$HOME/.config/maa" "$(maa dir config)"
```

在下面的例子中，我们假设配置目录是`$MAA_CONFIG_DIR`。

### MAA设置

MAA的设置储存在`$MAA_CONFIG_DIR/asst.json`或者`$MAA_CONFIG_DIR/asst.toml`中。
这个文件包含两个部分：`[connection]`和`[instance_options]`。

`[connection]`部分用于MAA连接游戏的设置，
其包括两种方式：通过ADB连接和通过PlayCover连接。
当你使用ADB连接时，你需要提供`adb`的路径和设备的序列号：
```toml
[connection]
type = "ADB" # or "PlayCover" 但是后者还没有实现
adb_path = "adb" # adb可执行文件的路径
device = "emulator-5554" # 你的android设备的序列号
config = "General" # maa connect的配置
```
当你使用PlayCover连接时，你需要提供在PlayCover中设置的MacTools的地址：
```toml
[connection]
type = "PlayCover"
client_address = "localhost:1717" # MacTools的地址
config = "CompatMac" # maa connect的配置
```
两者都需要提供`config`，这个值是`maa connect`的参数，
它指定了MAA连接游戏时使用的配置，
对于macOS上的MAA，它默认为`CompatMac`，对于其他平台的MAA，它默认为`General`，
具体可选值请参考MAA仓库中resource/config.json文件。

`[instance_options]`部分用于配置MAA实例的选项：
```toml
[instance_options]
touch_mode = "ADB" # 使用的触摸模式，可选值为"ADB", "MiniTouch", "MaaTouch"  或者 "MacPlayTools"(仅适用于PlayCover)
deployment_with_pause = false # 是否在部署时暂停游戏
adb_lite_enabled = false # 是否使用adb-lite
kill_adb_on_exit = false # 是否在退出时杀死adb
```
注意，如果你使用`PlayTools`，`touch_mode`字段将被忽略并被设置为`MacPlayTools`。

`resources`部分用于指定资源的路径，这是一个资源目录列表（路径应该相对于MAA仓库的`resource`目录）：
```toml
resources = ["platform_diff/macOS"]
```
这对于外服和平台特定游戏资源就很有用。

### 自定义任务

每一个任务都是一个单独的文件，它们储存在`$MAA_CONFIG_DIR/tasks`中。
任务文件的格式是`<name>.toml`或者`<name>.json`，其中`<name>`是任务的名字。

一个任务文件包含多个子任务，每一个子任务是一个[MAA任务链](https://maa.plus/docs/3.1-集成文档.html#asstappendtask)：
```toml
[[tasks]]
type = "StartUp" # maa任务的类型
params = { client_type = "Official", start_game_enabled = true } # maa任务的参数
```
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
这里的`condition`字段用于确定哪一个变体应该被使用，
而匹配的变体的`params`字段将会被合并到任务的参数中。

**注意**：这个CLI不会读取基建计划文件中的任何内容，
包括基建计划文件中定义的时间段，
所以你必须在`condition`字段中定义时间段，
来在不同的时间运行不同的基建计划。

除了`Time`条件，还有`DateTime`和`Weakday`条件：
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
如果有多个变体被匹配，第一个将会被使用。
如果没有给出条件，那么变体将会总是被匹配，
所以你可以把没有条件的变体放在最后，作为默认的情况。

如果没有变体被匹配，那么任务将不会被执行，
这在你想要只在某些条件下运行任务时很有用：
```toml
# 只在在18:00:00之后进行信用商店相关的操作
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

配置文件的例子可以在[`config_examples`目录](./config_examples)中找到。
另一个例子是我自己的配置文件，你可以在[这里](https://github.com/wangl-cc/dotfiles/tree/master/.config/maa)找到。

### 处理MAA消息

当运行任务时，它会处理MAA的消息。但是不是所有的消息都会输出。
日志级别用于控制哪些消息会被输出。
当前CLI有6个日志级别：
- Error：出现错误，程序可能会退出或者不能正常工作；
- Warning：出现错误，但是程序仍然可以正常工作；
- Normal：一些重要的信息，例如任务开始和结束；
- Info：更详细的信息，例如关卡掉落；
- Debug：关于你的配置的详细信息，例如将要运行任务的参数，
  这对于你调试你的配置很有用；
- Trace：任何没有被CLI处理的MAA消息，主要用于开发者调试CLI很有用。

默认的日志级别是`Normal`，你可以通过`-v`和`-q`选项来控制日志级别：
`-v`将会提高日志级别以显示更多消息，`-q`将会减少日志级别以显示更少的消息。

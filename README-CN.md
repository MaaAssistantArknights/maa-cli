# maa-cli

一个使用rust编写的简单[MAA](https://github.com/MaaAssistantArknights/MaaAssistantArknights)命令行工具。

## 功能

- 通过TOML和JSON文件定义MAA任务，并通过`maa run <task>`执行；
- 基于GUI的Callback实现的消息处理(尚未完全实现)，更好的监测MAA运行状态。

## 安装

这是一个使用Rust编写的命令行工具，所以你必须[安装Rust](https://www.rust-lang.org/tools/install)并确保`cargo`可用。

### 安装`MaaCore`及相关资源

`MaaCore`是构建`maa-sys`所必需的共享库，所以你必须在安装这个CLI之前安装`MaaCore`。最简单的方法应该是`maa-updater`，但它还没有实现，所以你现在必须自己完成。

对于macOS用户，如果你已经安装了`MAA.app`在`/Applications/MAA.app`，那么库可以在`/Applications/Maa.app/Contents/Frameworks`找到，资源可以在`/Applications/Maa.app/Contents/Resources/resource`找到。然后你可以将它们链接到需要的位置：
```bash
if [ -n "$XDG_DATA_HOME" ]; then # maa-cli respect XDG Base Directory Specification
    MAA_DATA_DIR="$XDG_DATA_HOME/maa"
else
    MAA_DATA_DIR="$HOME/Library/Application\ Support/com.loong.maa"
fi
ln -s /Applications/Maa.app/Contents/Frameworks "$MAA_DATA_DIR/lib"
ln -s /Applications/Maa.app/Contents/Resources/resource "$MAA_DATA_DIR/resource"
```

对于Linux，你应该从[这里](https://maa.plus)下载最新版本的`MAA`。然后，如果你已经将`MAA`下载到了`~/Downloads`，那么你可以将它解压到正确的位置：
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

对于Windows用户，我不知道该怎么做，因为我没有Windows机器。
如果你想尝试可以先安装`maa-updater`：
```bash
cargo install --git https://github.com/wangl-cc/maa-cli maa-updater --locked
```
然后使用`maa-updater package`，他应该会告诉你库和资源应该安装的的位置。

### 安装`maa-cli`

如果你根据上面的步骤将MAA库和资源安装到了正确的位置，那么你可以使用`cargo`安装`maa-cli`：
```bash
cargo install --git https://github.com/wangl-cc/maa-cli maa-cli --locked
```

## 使用

`maa-cli`可以运行你定义的任务（具体如何定义后面会讲到）：
```bash
maa-cli run <task> [options]
```
更多的命令可以通过`maa-cli --help`查看。

### 配置文件夹

MAA相关的设置和定义的任务都储存在配置文件夹里，
在Linux上，它应该在`~/.config/maa`，在macOS上，它应该在`~/Library/Application Support/com.loong.maa/config`。
这个文件夹可以通过设置环境变量`MAA_CONFIG_DIR`来改变，或者通过设置`XDG_CONFIG_HOME`来改变`MAA_CONFIG_DIR`的默认值。
在以下的文档中，我们将使用`$MAA_CONFIG_DIR`来表示配置文件夹。

### MAA设置

MAA的设置储存在`$MAA_CONFIG_DIR/asst.json`或者`$MAA_CONFIG_DIR/asst.toml`中。
这个文件包含两个部分：`[connection]`和`[instance_options]`。

`[connection]`部分用于MAA连接游戏的设置：
```toml
[connection]
type = "ADB" # or "PlayCover" 但是后者还没有实现
adb_path = "adb" # adb可执行文件的路径
device = "emulator-5554" # 你的android设备的序列号
config = "General" # maa connect的配置，macOS上默认为`CompatMac`，其他平台默认为`General`，具体可选值请参考MAA仓库中resource/config.json文件
```

`[instance_options]`部分用于配置MAA实例的选项：
```toml
[instance_options]
touch_mode = "ADB" # 使用的触摸模式，可选值为"ADB", "MiniTouch", "MaaTouch"  或者 "MacPlayTools"(目前不可用)
deployment_with_pause = false # 是否在部署时暂停游戏
adb_lite_enabled = false # 是否使用adb-lite
kill_adb_on_exit = false # 是否在退出时杀死adb
```
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

完整的例子请参考我的[dotfiles](https://github.com/wangl-cc/dotfiles/tree/master/.config/maa).

## 未完成的功能

- [ ] maa-updater
- [ ] 更好的消息处理
  - [ ] 肉鸽相关消息处理
  - [ ] Subtask extra info消息处理
- [ ] PlayCover支持

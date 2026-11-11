---
name: maa-cli
description: >-
  Operate MaaAssistantArknights maa-cli (`maa` / `maa-cli`) for 明日方舟 automation.
  Use when installing or running maa-cli, connecting via PlayCover / Waydroid / ADB,
  or diagnosing a failed game connection. Do not use for the MAA desktop GUI,
  non-Arknights games, or this repo's Rust/CI work.
license: AGPL-3.0-only
metadata:
  author: MaaAssistantArknights
  version: "0.2.2"
  tags: arknights maa-cli maacore
---

# maa-cli

maa-cli 只服务《明日方舟》。它把任务交给已安装的 MaaCore，自己不识图、不点屏幕。
本机命令多数是 `maa`；Windows winget 安装后是 `maa-cli`。先 `command -v maa`，没有再试 `maa-cli`。下文示例写 `maa`。
旗标以 `maa help` / `maa help <command>` 为准。

**会点击购买或消耗源石的操作，先得到用户明确同意。**

## 三层

会操作游戏的命令要求三层同时就绪。缺哪一层就补哪一层。

1. **maa-cli**：PATH 上的 `maa` 或 `maa-cli`。
2. **MaaCore + 资源**：`maa install`（已装则 `maa update`）。热更新资源用 `maa hot-update`。Nix 已捆绑 Core。包管理器装的 cli 用包管理器升级。
3. **游戏运行时**：Android 走 ADB（模拟器、真机、Waydroid）；macOS 上的 iOS 包走 PlayCover + PlayTools。maa-cli 不安装模拟器、PlayCover 或游戏。

数量、余量、限购只引用命令日志或用户给出的数字。物品 ID 查已装资源的 `item_index.json`（`maa dir resource`）。

## 冷启动

完成标准：`maa version` 同时打出 cli 与 Core；`maa dir config` 能打印路径；若要跑游戏，profile 的 preset/地址与当前运行时一致，且目标命令 `--dry-run` 通过。

1. 跑 `maa version`。命令不存在 → 读 [setup.md](setup.md) 的「安装 maa-cli」，装到 `maa version` 能跑。没有 Core → `maa install`（Windows 先装 VC++）。**完成：** 输出版本里同时有 cli 和 Core。
2. `maa dir config` 打印配置目录（`MAA_CONFIG_DIR` 可覆盖）。**完成：** 得到一条真实路径。
3. 要操作游戏且运行时不明：问用户是 PlayCover、哪家模拟器、Waydroid 还是真机，再按 [setup.md](setup.md) 写 `$MAA_CONFIG_DIR/profiles/default.toml`。有人在场可用 `maa init`；无人值守直接写文件。**完成：** profile 的 preset/地址与当前客户端一致，或用户只要安装、不连游戏。
4. `maa startup --dry-run`（或目标任务加 `--dry-run`）。连不上走 setup 的连通性检查。**完成：** dry-run 退出码为 0。
5. 任务文件有输入项时加 `--batch`。**完成：** 不会在提示符上卡住。

## 连接怎么选

选 `preset` 用下表。写入文件的完整模板、分平台依赖、排错见 [setup.md](setup.md) 对应平台小节和「写连接 profile」。

|本机|游戏怎么跑|`connection.preset`|默认地址|
|---|---|---|---|
|macOS|PlayCover 安装的 iOS 包|`PlayCover`|`127.0.0.1:1717`（以窗口标题 `[host:port]` 为准）|
|macOS|MuMu Player Pro|`MuMuPro`|`127.0.0.1:16384`|
|Windows|腾讯 Androws|`Androws`|`127.0.0.1:5555`|
|Linux|Waydroid|`Waydroid`|`waydroid`|
|任意|其它 Android 模拟器或真机|`ADB`|`adb devices` 的 serial，找不到则 `emulator-5554`|

PlayCover 预设强制 `MacPlayTools`，并加载 iOS 资源。窗口保持可见。B 服没有 iOS 包。
本次运行覆盖地址：`maa fight 1-7 --addr 127.0.0.1:1717`。

## 命令怎么路由

|用户要|去做|
|---|---|
|装 / 更新 Core|`maa install` / `maa update`|
|热更新资源|`maa hot-update`|
|路径 / 版本 / 活动表|`maa dir …` / `maa version` / `maa activity`|
|启动、作战、抄作业、肉鸽、生息|预定义子命令：`startup` `closedown` `fight` `copilot` `ssscopilot` `paradoxcopilot` `roguelike` `reclamation`|
|基建、公招、信用商店、领奖、仓库、干员箱|自定义任务的 `type`，`maa run <文件名不含扩展名>`|

预定义例子：`maa startup Official && maa fight BB-7 -m 3 && maa closedown`
作业 URI 用 `prts://<id>` 或 `prts://s<id>`，或本地 JSON。
自定义任务放 `$MAA_CONFIG_DIR/tasks/`。类型名与识别边界见 [reference.md](reference.md)。
`Mall` 的 `buy_first` / `blacklist` 会在信用商店真实下单。

## 何时读哪份文件

- 缺 cli / Core、要写 profile、连不上：读 [setup.md](setup.md) 里对应当前 OS 的那一节。
- 自定义 `type`、URI、仓库/干员箱能扫到什么：读 [reference.md](reference.md) 对应小节。

仓库内用户手册：

- 安装：[install.md](../../crates/maa-cli/docs/zh-CN/install.md)
- 使用：[usage.md](../../crates/maa-cli/docs/zh-CN/usage.md)
- 配置：[config.md](../../crates/maa-cli/docs/zh-CN/config.md)

## 常见错误

|错|改成|
|---|---|
|把 `Infrast` / `Recruit` / `Mall` / `Depot` 当成子命令|自定义任务 `type` + `maa run …`|
|只装了 cli 就开打|`maa version` 看到 Core 之后再连|
|macOS 默认 ADB 去连 PlayCover|preset `PlayCover`，地址用标题栏端口|
|作业写成 `maa://`|`prts://`|
|把 `Mall` 名单当成推荐清单|先确认再跑；名单里的东西会被买掉|
|把 `maa activity` 当成背包|活动表来自已装资源；背包走 `Depot` 任务日志|

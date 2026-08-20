---
name: maa-cli
description: >-
  Operate MaaAssistantArknights maa-cli (command `maa` / `maa-cli`): install MaaCore,
  run Fight/Infrast/Recruit/Mall/Copilot tasks, read dirs and activity, and explain
  Depot/OperBox OCR limits. Use when the user mentions maa-cli, maa install, maa run,
  maa fight, MaaCore, 明日方舟自动化, 仓库识别, 干员箱, or combining maa-cli with an agent.
---

# maa-cli 使用

面向 **调用 maa-cli 跑任务 / 查状态** 的 agent，不是改本仓库源码。
改 crate、CI、FFI 请看仓库根目录 `AGENTS.md`。
不要靠 `maa --help` 猜能力边界；命令语法细节以 `maa help <command>` 和 [reference.md](reference.md) 为准。

Windows 上用 winget 安装时，命令名是 `maa-cli`，下文的 `maa` 一律替换即可。

## 先分清三层数据

|平面|含义|要不要开游戏|
|---|---|---|
|catalog|静态表：MaaResource（`item_index.json` 等）或 ArknightsGameData|否|
|account|截图 OCR：仓库 Depot、干员箱 OperBox|要模拟器/客户端|
|action|点击游戏：Fight、基建、公招、信用商店|要|

查技能数值、攻防、专精材料 → catalog（GameData），**不是** MaaResource，更不是 OCR。
查「我有多少固源岩」→ account（Depot）。
「帮我刷 1-7」→ action。

## 最小前置

1. 二进制可用：`maa version`。
2. MaaCore 已装：`maa install`（已装则 `maa update`）。资源热更新：`maa hot-update`。
3. 需要跑游戏任务时，设备已连接（ADB / PlayCover 等），配置用 `maa init` 或 `$MAA_CONFIG_DIR` 下的 profile。
4. 配置目录：`maa dir config`（可用 `MAA_CONFIG_DIR` 覆盖）。

未装 Core 时不要编造任务结果。路径一律用 `maa dir …`，不要写死 `/usr/local` 或 `C:\Program Files`。

## 命令地图

### 管理

- `maa install` / `maa update`：安装或更新 MaaCore 与资源。
- `maa self update`：更新 maa-cli 自身；包管理器安装的用户不要用这条。
- `maa hot-update`：热更新资源仓库，不替换 Core 自带的基础资源。
- `maa init`：交互初始化 profile。
- `maa version` / `maa dir <data|library|config|cache|resource|hot-update|log>`。

### 预定义任务（会操作游戏）

- `maa startup [client]` / `maa closedown [client]`
- `maa fight [stage]`（如 `1-7`；空则上次/当前关）
- `maa copilot <uri>...`：作业 URI 用 `prts://<id>` 或 `prts://s<id>` 或本地 JSON；`maa://` 已弃用。
- `maa sscopilot` / `maa paradoxcopilot` / `maa roguelike <theme>` / `maa reclamation <theme>`

### 自定义任务

- 文件在 `$MAA_CONFIG_DIR/tasks/`，`maa run <文件名不含扩展名>`。
- `maa list` 列出可用任务。
- Depot / OperBox **没有** `maa depot` 这种预设子命令，只能写自定义任务，`type` 为 `Depot` 或 `OperBox`。
- 任务类型与参数以 [MAA 集成文档](https://github.com/MaaAssistantArknights/MaaAssistantArknights/blob/dev/docs/zh-cn/protocol/integration.md) 为准；maa-cli **不校验** 参数名，错了要等 Core 运行时才爆。

### 只读 / 工具

- `maa activity [client]`：当前活动关卡提示（catalog + 资源，不 OCR 背包）。
- `maa convert`、`maa complete`、`maa cleanup`、`maa import`、`maa remainder`。

参数示例：`maa fight BB-7 -m 3` 用 3 瓶理智药。
日常串联：`maa startup Official && maa fight 1-7 -m 3 && maa closedown`。

## 安全与默认策略

- **信用商店 `Mall` 会真的下单。** 自定义任务里的 `buy_first` / `blacklist` 是执行规则，不是建议。
- **不要代买红票/黄票/绿票/情报凭证商店。** 当前 maa-cli 也没有这些店的只读扫描命令。若用户要购买建议：用 catalog 货盘 + 已有 Depot 材料，输出清单，默认不 click。
- 不要把 OCR 快照当成完整账号资产。边界见 [reference.md](reference.md)。
- 需要确认才执行会消耗源石（`--stone`）或清空理智的长时间 Fight。

## 给 agent 的输出习惯

跑任务后据实汇报：用了哪些命令、任务总结里的关卡/掉落/公招，以及 OCR 没覆盖的部分。
引用物品用 `item_index.json` 的 id（如固源岩 `30012`），不要只写俗称。
区分「Core 没这个功能」和「命令写错了」。

## 人类文档

- 安装：[docs/zh-CN/install.md](../../crates/maa-cli/docs/zh-CN/install.md)
- 使用：[docs/zh-CN/usage.md](../../crates/maa-cli/docs/zh-CN/usage.md)
- 配置与自定义任务：[docs/zh-CN/config.md](../../crates/maa-cli/docs/zh-CN/config.md)
- 示例任务：`crates/maa-cli/config_examples/tasks/daily.json`
- 命令与 OCR 边界：[reference.md](reference.md)

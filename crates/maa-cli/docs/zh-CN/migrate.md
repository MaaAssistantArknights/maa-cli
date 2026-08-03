# 配置迁移

`maa migrate` 用于把已有配置迁移到当前 maa-cli 所使用的配置形式。

不同来源对应不同的子命令。本文按迁移来源分节说明；目前仅完成 WPF GUI，其他来源会在后续补充。

## 目录

- [WPF GUI](#wpf-gui)
- [config-v1 到 config-v2](#config-v1-到-config-v2)（TODO）

## WPF GUI

把 MAA WPF GUI 的配置迁移为 maa-cli 任务配置：

```bash
maa migrate wpf <input> [output] [-f toml|yaml|json] [--config <name>]
```

`<input>` 应指向 GUI 导出的配置文件：

| 平台 | 输入文件 |
| --- | --- |
| Windows | `config/gui.new.json`（相对 MAA 安装或用户配置目录；以你本机实际路径为准） |
| Linux | TODO：待补充路径与测试 |
| macOS | TODO：待补充路径与测试 |

如果文件中有多套配置，可用 `--config` 或环境变量 `MAA_GUI_CONFIG` 指定名称；未指定时会交互选择，默认使用文件中的当前配置（`Current`）。

迁移可能不完整：不支持的任务类型会被跳过；GUI 中已关闭的任务会保留在结果里，但默认不会执行；未能对应的参数会出现在迁移摘要中。命令结束后会输出这些汇总，方便你对照检查。

### StartUp

对应 GUI 任务队列中的启动任务，迁移为 maa-cli 的 `StartUp` 任务。

| GUI 字段 | maa-cli 字段 | 规则 |
| --- | --- | --- |
| `Configurations.<配置名>.Gui.RuntimeSettings.ClientType` | `tasks[].params.client_type` | 合法值（`Official` / `Bilibili` / `txwy` / `YoStarEN` / `YoStarJP` / `YoStarKR`）直接写入；缺失或不合法时将写入备选值，备选值将在 maa-cli 运行时交互确认最终值 |
| `Configurations.<配置名>.Gui.RuntimeSettings.StartGame` | `tasks[].params.start_game_enabled` | 有值时写入 |
| `Configurations.<配置名>.TaskQueue[] \| {AccountSwitchEnabled, AccountName}` | `tasks[].params.account_name` | 仅当 GUI 配置 `AccountSwitchEnabled` 为 `true` 时读取 `AccountName` 并尝试写入迁移配置 `account_name` |

TODO：`account_name` 的转换**我没有测试过**（未做实机账号切换验证）。

### Fight

对应 GUI 任务队列中的战斗任务，迁移为 maa-cli 的 `Fight` 任务。
`Name` 会写入任务的 `name`；其余公共参数写入 `params`（若存在 `variants`，则写入每个变体的 `params`）。

| GUI 字段 | maa-cli 字段 | 规则 |
| --- | --- | --- |
| `Name` | `name` | 有值时写入 |
| `UseMedicine` + `MedicineCount` | `params.medicine` | 仅当 `UseMedicine` 为 `true` 时写入 `MedicineCount` |
| `UseStone` + `StoneCount` | `params.stone` | 仅当 `UseStone` 为 `true` 时写入 `StoneCount`；迁移时会警告该设置可能消耗源石 |
| `EnableTimesLimit` + `TimesLimit` | `params.times` | 仅当 `EnableTimesLimit` 为 `true` 时写入 `TimesLimit` |
| `EnableTargetDrop` + `DropId` + `DropCount` | `params.drops` | 仅当 `EnableTargetDrop` 为 `true` 时写入，格式为 `{ <DropId> = <DropCount> }` |
| `Series` | `params.series` | 仅当值不为 `0` 时写入 |
| `UseExpiringMedicine` + `MedicineExpireDays` | `params.medicine_expire_days` | 仅当 `UseExpiringMedicine` 为 `true` 时写入 `MedicineExpireDays` |
| `StagePlan` | `params.stage` | 见下方「关卡与变体」 |

#### 关卡与变体

关卡与条件变体由 `UseWeeklySchedule` 与 `UseOptionalStage` 共同决定。

| `UseWeeklySchedule` | `UseOptionalStage` | `StagePlan` 要求 | 输出结构 |
| --- | --- | --- | --- |
| `false` | `false` | 单个关卡（字符串，或仅含一个字符串的数组） | 无 `variants`，关卡写入顶层 `params.stage` |
| `true` | `false` | 同上 | 一个 `variants`；`condition` 为周计划对应的 `Weekday`；关卡写入该变体的 `params.stage` |
| `false` | `true` | 非空字符串数组 | 每个关卡一个 `variants`；`condition` 由关卡开放条件决定 |
| `true` | `true` | 非空字符串数组 | 每个关卡一个 `variants`；`condition` 为 `And`，依次合并周计划与关卡开放条件 |

周计划来自 `WeeklySchedule`：GUI 中为 `true` 的星期会写入 `condition = { type = "Weekday", weekdays = [...] }`（如 `Sun` / `Mon` / … / `Sat`）。
若 `UseWeeklySchedule` 为 `true` 但缺少有效的 `WeeklySchedule`，迁移会报错退出。
若 `UseOptionalStage` 为 `false` 但 `StagePlan` 含多个关卡，迁移会报错退出。

启用 `UseOptionalStage` 时，每个关卡的开放条件大致如下：

| 关卡 | 条件 |
| --- | --- |
| `CE-6` | `Weekday`：`Tue` / `Thu` / `Sat` / `Sun`（`timezone = "Official"`） |
| `AP-5` | `Weekday`：`Mon` / `Thu` / `Sat` / `Sun`（`timezone = "Official"`） |
| `CA-5` | `Weekday`：`Tue` / `Wed` / `Fri` / `Sun`（`timezone = "Official"`） |
| `SK-5` | `Weekday`：`Mon` / `Wed` / `Fri` / `Sat`（`timezone = "Official"`） |
| `PR-A-1` / `PR-A-2` | `Weekday`：`Mon` / `Thu` / `Fri` / `Sun`（`timezone = "Official"`） |
| `PR-B-1` / `PR-B-2` | `Weekday`：`Mon` / `Tue` / `Fri` / `Sat`（`timezone = "Official"`） |
| `PR-C-1` / `PR-C-2` | `Weekday`：`Wed` / `Thu` / `Sat` / `Sun`（`timezone = "Official"`） |
| `PR-D-1` / `PR-D-2` | `Weekday`：`Tue` / `Wed` / `Sat` / `Sun`（`timezone = "Official"`） |
| `LS-6` / `Annihilation` / `OF-1` / `OF-F3` | `Always` |
| 热更新 `StageActivityV2.json` 中的侧边故事关卡 | `OnSideStory`（TODO: 暂时只按官服活动表判断） |
| 其他未识别关卡（如主线） | `Always`，并输出警告 |

示例：仅启用可选关卡、刷 `CE-6` 与 `LS-6` 时，迁移结果类似：

```toml
[[tasks]]
type = "Fight"
name = "理智作战"

[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat", "Sun"], timezone = "Official" }
params = { medicine_expire_days = 2, stage = "CE-6" }

[[tasks.variants]]
condition = { type = "Always" }
params = { medicine_expire_days = 2, stage = "LS-6" }
```

## config-v1 到 config-v2

TODO

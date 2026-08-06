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

迁移可能不完整：不支持的任务类型会被跳过；GUI 中已关闭的任务（`IsEnable = false`）会保留，并写入 `params.enable = false`；未能对应的参数会出现在迁移摘要中。命令结束后会输出这些汇总，方便你对照检查。

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
| `IsEnable` | `params.enable` | 为 `false` 时写入 `enable = false`（MaaCore 通用开关；默认不写，等同于 `true`） |
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

### Infrast

对应 GUI 任务队列中的基建换班任务，迁移为 maa-cli 的 `Infrast` 任务。
`Name` 会写入任务的 `name`；其余字段写入 `params`。

自定义基建**暂不支持**：若 `Mode` 为 `Custom`，或 `Filename` 非空，迁移会报错退出。
`PlanSelect` 不会写入迁移结果（默认值会被忽略；自定义计划本身亦不可用）。

| GUI 字段 | maa-cli 字段 | 规则 |
| --- | --- | --- |
| `Name` | `name` | 有值时写入 |
| `Mode` | `params.mode` | `Normal` → `0`；`Rotation` → `20000`；`Custom` → 报错；其他未知值记入迁移摘要 |
| `RoomList[].Room` | `params.facility` | 按顺序收集房间名数组（如 `Mfg` / `Trade` / `Dorm`） |
| `UsesOfDrones` | `params.drones` | 有值时写入（如 `Money`） |
| `DormThreshold` | `params.threshold` | GUI 为 0–100 的整数百分比，写入时除以 `100` 转为浮点（如 `30` → `0.3`） |
| `OriginiumShardAutoReplenishment` | `params.replenish` | 有值时写入 |
| `DormFilterNotStationed` | `params.dorm_notstationed_enabled` | 有值时写入 |
| `DormTrustEnabled` | `params.dorm_trust_enabled` | 有值时写入 |
| `ReceptionMessageBoard` | `params.reception_message_board` | 有值时写入 |
| `ReceptionClueExchange` | `params.reception_clue_exchange` | 有值时写入 |
| `SendClue` | `params.reception_send_clue` | 有值时写入 |

示例：

```toml
[[tasks]]
type = "Infrast"
name = ""

[tasks.params]
mode = 0
facility = ["Mfg", "Trade", "Control", "Power", "Reception", "Office", "Dorm", "Processing", "Training"]
drones = "Money"
threshold = 0.3
replenish = true
dorm_notstationed_enabled = true
dorm_trust_enabled = true
reception_message_board = true
reception_clue_exchange = true
reception_send_clue = true
```

### Recruit

对应 GUI 任务队列中的公开招募任务，迁移为 maa-cli 的 `Recruit` 任务。

| GUI 字段 | maa-cli 字段 | 规则 |
| --- | --- | --- |
| `Name` | `name` | 有值时写入 |
| `MaxTimes` | `params.times` | 有值时写入 |
| `ExtraTagMode` | `params.extra_tags_mode` | 有值时写入 |
| `RefreshLevel3` | `params.refresh` | 有值时写入 |
| `ForceRefresh` | `params.expedite` | 有值时写入 |
| `Level3Choose` / `Level4Choose` / `Level5Choose` / `Level6Choose` | `params.select` / `params.confirm` | 为 `true` 的星级会同时写入 `select` 与 `confirm`（如 `3` / `4` / `5`） |
| `Level3Time` / `Level4Time` / `Level5Time` / `Level6Time` | `params.recruitment_time` | 有值时写入对应星级的招募时长（分钟），如 `{ "3" = 540, "4" = 540 }` |
| `PreferTagEnabled` + `Level3PreferTags` | `params.first_tags` | 仅当 `PreferTagEnabled` 为 `true` 且标签列表非空时写入 |
| `PreserveTagEnabled` + `PreserveTagList` | `params.preserve_tags` | 仅当 `PreserveTagEnabled` 为 `true` 且标签列表非空时写入 |

### Mall

对应 GUI 任务队列中的信用商店任务，迁移为 maa-cli 的 `Mall` 任务。

| GUI 字段 | maa-cli 字段 | 规则 |
| --- | --- | --- |
| `Name` | `name` | 有值时写入 |
| `Shopping` | `params.shopping` | 有值时写入 |
| `CreditFight` | `params.credit_fight` | 有值时写入 |
| `CreditFightFormation` | `params.formation_index` | 有值时写入 |
| `VisitFriends` | `params.visit_friends` | 有值时写入 |
| `FirstList` | `params.buy_first` | 以 `;` 分隔的字符串拆成数组（空段丢弃） |
| `BlackList` | `params.blacklist` | 同上 |
| `ShoppingIgnoreBlackListWhenFull` | `params.force_shopping_if_credit_full` | 有值时写入 |
| `OnlyBuyDiscount` | `params.only_buy_discount` | 有值时写入 |
| `ReserveMaxCredit` | `params.reserve_max_credit` | 有值时写入 |

`CreditFightOnceADay` / `VisitFriendsOnceADay` 及对应的上次执行时间等字段目前不会迁移，有实际取值时会出现在迁移摘要中。

### Award

对应 GUI 任务队列中的领取奖励任务，迁移为 maa-cli 的 `Award` 任务。

| GUI 字段 | maa-cli 字段 | 规则 |
| --- | --- | --- |
| `Name` | `name` | 有值时写入 |
| `Award` | `params.award` | 日常任务奖励 |
| `Mail` | `params.mail` | 邮件 |
| `FreeGacha` | `params.recruit` | 免费十连 |
| `Orundum` | `params.orundum` | 合成玉 |
| `Mining` | `params.mining` | 限时开采 |
| `SpecialAccess` | `params.specialaccess` | 月卡等特殊访问 |

上述布尔字段有值时原样写入。

### Roguelike

对应 GUI 任务队列中的集成战略任务，迁移为 maa-cli 的 `Roguelike` 任务。

| GUI 字段 | maa-cli 字段 | 规则 |
| --- | --- | --- |
| `Name` | `name` | 有值时写入 |
| `Theme` | `params.theme` | 有值时写入（如 `JieGarden`） |
| `Mode` | `params.mode` | `Exp` → `0`；`Investment` → `1`；`Collect` → `4`；`CollapsalParadigms` → `5`；`MonthlySquad` → `6`；`DeepExploration` → `7`；未知值记入迁移摘要 |
| `Squad` | `params.squad` | 有值时写入 |
| `Roles` | `params.roles` | 有值时写入 |
| `CoreChar` | `params.core_char` | 有值时写入 |
| `StartCount` | `params.starts_count` | 有值时写入 |
| `Difficulty` | `params.difficulty` | 有值且不为 `2147483647`（`i32::MAX`，表示未指定）时写入 |
| `Investment` | `params.investment_enabled` | 有值时写入 |
| `InvestCount` | `params.investments_count` | 有值时写入 |
| `InvestWithMoreScore` | `params.investment_with_more_score` | 有值时写入 |
| `StopWhenDepositFull` | `params.stop_when_investment_full` | 有值时写入 |
| `StopAtFinalBoss` | `params.stop_at_final_boss` | 有值时写入 |
| `StopWhenLevelMax` | `params.stop_at_max_level` | 有值时写入 |
| `UseSupport` | `params.use_support` | 有值时写入 |
| `UseSupportNonFriend` | `params.use_nonfriend_support` | 有值时写入 |
| `RefreshTraderWithDice` | `params.refresh_trader_with_dice` | 有值时写入 |
| `StartWithEliteTwo` | `params.start_with_elite_two` | 有值时写入 |
| `StartWithEliteTwoOnly` | `params.only_start_with_elite_two` | 有值时写入 |

主题专属字段（如萨米密文板、月度分队自动迭代、种子开局等）目前不会迁移，有实际取值时会出现在迁移摘要中。

### Reclamation

对应 GUI 任务队列中的生息演算任务，迁移为 maa-cli 的 `Reclamation` 任务。

| GUI 字段 | maa-cli 字段 | 规则 |
| --- | --- | --- |
| `Name` | `name` | 有值时写入 |
| `Theme` | `params.theme` | 有值时写入（如 `Tales`） |
| `Mode` | `params.mode` | `ProsperityNoSave` → `0`；`ProsperityInSave` → `1`；未知值记入迁移摘要 |
| `ToolToCraft` | `params.tools_to_craft` | 非空时写入单元素数组 |
| `IncrementMode` | `params.increment_mode` | 有值时写入 |
| `MaxCraftCountPerRound` | `params.num_craft_batches` | 有值时写入 |

其余未映射字段（如 `ClearStore`）有实际取值时会出现在迁移摘要中。

## config-v1 到 config-v2

TODO

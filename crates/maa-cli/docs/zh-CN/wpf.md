# 从 WPF GUI 迁移配置

`maa migrate wpf` 可以把 MAA Windows WPF GUI 导出的配置文件，转换为 maa-cli 的自定义任务配置。
迁移是有损的：不支持的任务类型与字段会被跳过，并在结束后输出摘要；转换完成后，你通常还需要对照 [配置文档](config.md) 做一次检查与微调。

## 使用方法

输入一般是 GUI 导出的 JSON 配置文件（包含 `Configurations` / `TaskQueue`）。

```bash
# 输出默认写到与输入同名的 .toml 文件
maa migrate wpf gui.new.json

# 指定输出路径与格式（由扩展名决定，支持 toml / yaml / json）
maa migrate wpf gui.new.json tasks/daily.toml

# 多配置导出时，用 --profile-name 选择配置名
maa migrate wpf gui.new.json --profile-name Default

# GUI 是自定义排班时，用本机排班 JSON 覆盖其中的 Filename
maa migrate wpf gui.new.json --custom-schedule ~/一图流-153-一天两换-MAA.json
```

如果导出文件里只有一个配置，会自动选择该配置；有多个配置且未指定 `--profile-name` 时，会交互式提示你选择。

`--custom-schedule` 只在 GUI 基建任务已经是自定义排班（`Mode = Custom` 或填写了 `Filename`）时生效，用来覆盖排班文件路径。若 GUI 不是自定义排班，传入该选项会报错退出。

迁移完成后，把生成的任务文件放到 `$MAA_CONFIG_DIR/tasks` 中，即可通过 `maa run <task>` 运行。配置目录可用 `maa dir config` 查看。

## 迁移结果

输出是一份 maa-cli 自定义任务文件，结构与 [自定义任务](config.md#自定义任务) 相同：顶层是 `[[tasks]]` 列表，每个任务包含 `type`、`name`、`params`，必要时还有 `variants`。

GUI 里 `IsEnable = false` 的任务仍会写出，但会带上 `params.enable = false`，因此默认不会执行；需要时再手动改回 `true` 或删掉该字段。

示例输入与输出可以参考仓库中的 [`config_examples/wpf`](../../config_examples/wpf)。

## 任务映射说明

### 开始唤醒（StartUp）

`client_type` 与 `start_game_enabled` 来自 GUI 的 `Gui.RuntimeSettings`，而不是 `StartUpTask` 本体。
如果 GUI 未填写可识别的客户端类型，迁移结果会把 `client_type` 写成交互式选项，运行任务时再选择。
若开启了账号切换但未填写账号名，该字段会被跳过并给出警告。

### 理智作战（Fight）

药品、源石、次数、掉落、连战、过期理智药等共享参数写在任务级 `params` 中；关卡相关内容写在变体里。

当 GUI 开启 `UseOptionalStage` 时，`StagePlan` 中的每个关卡会生成一个变体，并按关卡开放规则附上条件：

- 资源本 / 芯片本：`Weekday`（官服服务器时区）
- 永久关卡（如主线、剿灭、`LS-6`）：`Always`
- 活动关卡：若能在热更新资源 `StageActivityV2.json` 中查到对应活动，则写入 `DateTime`（使用该活动的截止时间与时区）；查不到则回退为 `Always` 并告警

例如 GUI 中同时启用过期理智药与多关卡备选时，迁移结果类似：

```toml
[[tasks]]
name = "理智作战"
type = "Fight"

[tasks.params]
medicine_expire_days = 2

[[tasks.variants]]
condition = { type = "DateTime", end = "2026-08-22T03:59:59", timezone = 8 }
params = { stage = "TO-5" }

[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat", "Sun"], timezone = "Official" }
params = { stage = "CE-6" }

[[tasks.variants]]
condition = { type = "Always" }
params = { stage = "LS-6" }
```

这里的 `medicine_expire_days` 对应 GUI 的 `MedicineExpireDays`（在 `UseExpiringMedicine = true` 时写入），表示使用多少天内将过期的理智药。
活动关卡的 `end` 来自迁移时本地的 `StageActivityV2.json`，之后不会随热更新自动刷新；活动换档后需重新迁移或手动改条件。
若同时启用 `UseWeeklySchedule`，周计划会与关卡开放条件用 `And` 组合。

**注意**：启用源石（`UseStone`）时会发出警告，因为这可能消耗源石。

### 公开招募（Recruit）

常见字段会映射为 maa-cli / MaaCore 参数，例如：

- `MaxTimes` → `times`
- `RefreshLevel3` → `refresh`
- `ExtraTagMode` → `extra_tags_mode`
- `LevelNChoose` / `LevelNTime` → `select` / `confirm` / `recruitment_time`
- 偏好标签、保留标签 → `first_tags` / `preserve_tags`

以下行为需要特别注意：

- GUI 的 `ForceRefresh` **不是** MaaCore 的 `expedite`，maa-cli 也不支持该功能。迁移时不会写出对应参数；若 GUI 中为开启状态，会记入跳过字段并警告，该设置无效。
- 若 `Level6Choose = true`，会警告即将自动确认 6 星干员，这是危险行为。

### 基建换班（Infrast）

普通模式（`Normal` / `Rotation`）会迁移设施顺序、无人机用途、宿舍阈值等常见参数。
自定义基建计划（`Mode = Custom` 或填写了 `Filename`）会尝试读取排班 JSON：

- 读到 `plans` 且各班有 `period`、GUI 为时间轮换（`PlanSelect = -1`）时，写出 `mode = 10000`、`filename`，以及按时段（跨天时加上 `DayMod`）选择 `plan_index` 的变体。
- 读到排班但没有可用时段（或 GUI 选了固定班次）时，写出自定义模式并带上 `plan_index`。
- `Filename` 里的 `~` 会展开为用户主目录；写出的 `filename` 是展开后的绝对路径。
- 可用 `--custom-schedule <json>` 覆盖 GUI 中的排班文件路径；仅当 GUI 已是自定义排班时有效，否则报错退出。覆盖后若文件读失败，也会报错退出，而不会回退到默认基建。
- 未指定 `--custom-schedule` 时，文件不存在、无法解析或没有 `plans` 会跳过 `Mode` / `Filename`，按默认基建模式（`mode = 0`）写出其余参数，并在迁移摘要中说明。

### 其他任务

- `Mall`：信用商店、信用作战、访问好友、黑白名单等会映射到对应参数。
- `Award`：日常奖励、邮件、源石锭等；GUI 的 `FreeGacha` 对应 `recruit`。
- `Roguelike` / `Reclamation`：主题、模式与常见选项会迁移；无法识别的模式字段会记入跳过摘要。
- 未识别的 `$type` 会整项跳过，并出现在迁移摘要的「跳过任务」列表中。

各任务类型的完整参数含义请参考 [MAA 集成文档][task-types]。迁移后若要继续用条件变体精细控制执行逻辑，请参阅 [任务条件](config.md#任务条件)。

## 迁移摘要

迁移结束时，若存在有损处理，会在 stderr 打印摘要，例如：

- 跳过的任务（不支持的 `$type`）
- 已禁用但仍写出的任务（`params.enable = false`）
- 跳过的字段（无法映射或 maa-cli 不支持）

请根据摘要检查输出文件，确认行为符合预期后再用于日常任务。

## 参考

- [配置](config.md)
- [使用](usage.md)
- [示例：WPF 迁移][wpf-example]
- [MAA 集成文档：任务类型][task-types]

[wpf-example]: ../../config_examples/wpf
[task-types]: https://maa.plus/docs/zh-cn/protocol/integration.html#任务类型一览

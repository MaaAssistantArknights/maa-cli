# 从 WPF GUI 迁移配置

`maa migrate wpf` 把 MAA Windows 图形界面导出的 JSON 转成 maa-cli 任务文件。
不支持的任务和设置会被跳过。
结束时，命令把摘要写到标准错误。
转完后对照 [配置](config.md) 检查一遍，再用于日常任务。

## 迁移一份配置

输入是图形界面导出的 JSON 文件。

```bash
# 默认写到与输入同名的 .toml
maa migrate wpf gui.json

# 扩展名决定格式。可以是 toml、yaml 或 json
maa migrate wpf gui.json tasks/daily.toml

# 导出里有多份配置时，用名字选一份
maa migrate wpf gui.json --profile-name Default

# 图形界面里已经是自定义排班时，改用你给出的排班文件
maa migrate wpf gui.json --custom-schedule ~/base-schedule.json
```

导出里只有一份配置时，命令直接用它。
有多份，并且没有 `--profile-name` 时，命令会让你选。

`--custom-schedule` 只在图形界面里这份基建已经是自定义排班时有效。
还不是自定义排班时，加上这个选项会报错退出。

把生成的文件放到 `$MAA_CONFIG_DIR/tasks`，再用 `maa run <task>` 运行。
配置目录用 `maa dir config` 查看。

## 配置文档里已经有的设置

输出和 [自定义任务](config.md#自定义任务) 的结构相同。
顶层是 `[[tasks]]`。
每个任务有 `type`、`name` 和 `params`。
需要按条件切换时还有 `variants`。
条件的写法见 [任务条件](config.md#任务条件)。

下面只说明配置文档里已经出现过的名字。
配置文档没写、运行时直接交给 MaaCore 的名字，在下一节。

### 开始唤醒

`client_type` 和 `start_game_enabled` 就是配置文档启动示例里的那两个。
图形界面没有可识别的客户端时，`client_type` 会变成运行时再选的输入。
写法见配置文档的用户输入。
开了切换账号但没有账号名时，这项不写入。
摘要里会有一行跳过的字段。

### 理智作战

每个要打的关卡是一个变体。
关卡名写在 `stage`。

条件按关卡种类写。

- 资源本或芯片本用 `Weekday`，时区是官服。
- 活动关能在本机热更新的活动表里查到时，用 `DateTime`。里面有开始时间、结束时间和时区。查不到则用 `Always`，并告警。
- 主线、剿灭和 `LS-6` 这类常驻关用 `Always`。

`DateTime`、`Weekday` 和 `DayMod` 在配置文档的任务条件里。
`Always` 也是 maa-cli 的条件，表示不看时间，这个变体总是可以执行。
活动关的 `start` 和 `end` 取自迁移当时本机的活动表。
活动换档后不会自动更新。
需要重新迁移，或手改条件。
图形界面里还有一份每周计划时，周计划和关卡条件用 `And` 合在一起。

下面是多个备选关卡时的结果。

```toml
[[tasks]]
name = "理智作战"
type = "Fight"

[[tasks.variants]]
condition = { type = "DateTime", start = "2026-08-08T16:00:00", end = "2026-08-22T03:59:59", timezone = 8 }
params = { stage = "TO-5" }

[[tasks.variants]]
condition = { type = "Weekday", weekdays = ["Tue", "Thu", "Sat", "Sun"], timezone = "Official" }
params = { stage = "CE-6" }

[[tasks.variants]]
condition = { type = "Always" }
params = { stage = "LS-6" }
```

如果结果里会使用源石，命令会警告。
这个设置可能消耗源石。
理智药、源石、次数和过期药的参数名不在配置文档里，见下一节。

### 基建换班

设施顺序写在 `facility`。
宿舍是否填入信赖未满的干员写在 `dorm_trust_enabled`。
这两个名字在配置文档的基建例子里。

图形界面按时间自动换班，并且排班里每个班次都有时段时，`mode` 是 `10000`。
变体按时间段选择 `plan_index`。
跨天时段会加上 `DayMod`。
这和配置文档里的基建例子是同一套。
排班没有可用时段，或图形界面选了固定班次时，结果仍是自定义基建，并带上 `plan_index`。

写出的 `filename` 是绝对路径。
排班路径里的 `~` 展开成你的主目录。
相对路径按你传给命令的那份 JSON 所在目录解析。
配置文档说明，这份排班 JSON 由 MaaCore 读取，不由 maa-cli 读取。
排班文件里的菲亚梅塔也留在那个文件里，不抄进任务参数。

要换成另一份排班，用 `--custom-schedule` 给出那份 JSON 的路径。
图形界面还不是自定义排班时，命令报错退出。
指定的文件读失败时也报错退出，不会退回默认基建。

没有这个选项，并且排班不存在、无法解析或没有班次时，`mode` 写成 `0`。
摘要里会说明跳过了排班。

无人机、心情阈值、菲亚梅塔名单这些名字不在配置文档里，见下一节。

## 交给 MaaCore 的参数

[配置](config.md#基本结构) 写明，任务 `params` 里的具体名字以 MAA 集成文档为准。
maa-cli 不检查这些名字对不对。
运行时由 MaaCore 读取。
下面这些就是迁移会写进 `params`、但配置文档没有解释的名字。
要改数值，改这些名字。
每个名字的全部可选取值见 [MAA 集成文档][task-types]。

### 是否执行

`enable` 为 `false` 时，这个任务不执行。
图形界面里关掉的任务仍会写进文件，并带上 `enable = false`。
要让它跑，把该字段改成 `true`，或删掉它。

### 理智作战参数

- `medicine` 是理智药个数。配置文档只在用户输入的例子里出现过这个名字，没有说明它是理智药。
- `stone` 是源石个数。
- `times` 是次数上限。
- `series` 是连战次数。
- `drops` 是目标掉落。
- `medicine_expire_days` 是使用多少天内将过期的理智药。配置文档示例里的 `expiring_medicine` 是另一个名字，迁移不会写成那个名字。
- `DrGrandet` 为 `true` 时，在使用源石的确认界面等到回复 1 点理智，再立刻使用。

### 公开招募

- `times` 是招募次数。
- `refresh` 表示是否刷新三星标签。
- `extra_tags_mode` 是额外标签的处理方式。数字的含义见集成文档。
- `select` 和 `confirm` 是要自动选择、自动确认的星级，例如 `3`、`4`、`5`。
- `recruitment_time` 是各星级的招募分钟数。
- `first_tags` 是优先标签。
- `preserve_tags` 是要保留的标签。
- `level3_recruitment_permit_reserve` 是三星招募许可要保留的数量。这项没开，或数量不大于 0 时，不写这个名字。

图形界面里的立即刷新不会写入。
它开着时，摘要里会有一行跳过的字段，并警告。
开了自动确认六星时，命令会警告。

### 基建换班参数

- `drones` 是无人机用途。
- `threshold` 是宿舍心情阈值，范围是 0 到 1。图形界面里的百分比会换成这个小数。
- `replenish` 表示贸易站的源石碎片是否自动补货。
- `dorm_notstationed_enabled` 表示宿舍是否启用「未进驻」。
- `reception_message_board`、`reception_clue_exchange` 和 `reception_send_clue` 是会客室的留言板、线索交流和送出线索。
- `continue_training` 表示训练室是否继续训练。
- `fiammetta_targets` 是菲亚梅塔要换心情的干员名单。
- `fiammetta_recovery_enabled` 表示是否用菲亚梅塔恢复心情。
- `use_pinus_sylvestris`、`use_perception_information`、`use_worldly_plight` 和 `use_abyssal_hunter` 是红松、感知、尘世和深海这四组跨设施组合。为 `false` 时不写。

`fiammetta_targets` 只在 `mode` 为 0 且 `fiammetta_recovery_enabled` 为 true 时生效。
`fiammetta_recovery_enabled` 和上面四组组合只在 `mode` 为 0 时生效。

### 信用商店

- `shopping` 表示是否购买信用商品。
- `credit_fight` 表示是否进行信用作战。
- `formation_index` 是信用作战使用的编队。
- `visit_friends` 表示是否访问好友。
- `buy_first` 是优先购买的商品。
- `blacklist` 是不购买的商品。
- `force_shopping_if_credit_full` 表示信用满了是否仍按黑名单以外的商品购买。
- `only_buy_discount` 表示是否只买折扣商品。
- `reserve_max_credit` 表示是否把信用留到上限附近。

信用作战和访问好友的上次时间、今天是否还能做，是图形界面自己的记录。
这些不写入结果。

### 领奖

- `award` 是领取每日或每周任务奖励。
- `mail` 是领取邮件奖励。
- `recruit` 是领取限定池赠送的每日免费单抽。
- `orundum` 是领取幸运墙的合成玉。
- `mining` 是领取限时开采许可的合成玉。
- `specialaccess` 是领取五周年赠送的月卡。
- `signinevent` 是领取限时签到奖励。

### 肉鸽

这些配置未经实机验证。
相关参数的作用见 [MAA 集成文档][task-types]。

这些名字只在对应的主题和模式里有用。
写进去是为了留下你在图形界面里选过的内容。
MaaCore 在用不到的模式下会忽略它们。

- `theme` 是主题。
- `mode` 是模式编号。刷分是 `0`，刷源石锭是 `1`，凹开局是 `4`，坍缩范式是 `5`，月度小队是 `6`，深入调查是 `7`，刷常乐节点是 `20001`，黑流树海刷开局是 `30003`。认不出的模式不写 `mode`，并记入摘要。
- `squad` 是开局分队。`roles` 是开局职业组合。
- `core_char` 是第一名开局干员。`core_char_list` 是全部开局干员，每项有 `name` 和 `use_support`。
- `use_support` 表示第一名是否使用助战。`use_nonfriend_support` 表示是否使用非好友助战。
- `starts_count` 是开始次数。`difficulty` 是难度。难度如果是一个占位用的极大数字，则不写。
- `investment_enabled` 表示是否投资。`investments_count` 是投资次数。`investment_with_more_score` 表示是否用更高分数投资。`stop_when_investment_full` 表示存满后是否停止。
- `stop_at_final_boss` 表示是否在结局前停止。`stop_at_max_level` 表示等级满了是否停止。
- `start_with_elite_two` 和 `only_start_with_elite_two` 是精二开局。
- `refresh_trader_with_dice` 表示是否用骰子刷新商店。
- `collectible_mode_squad` 是凹开局或黑流树海准备局使用的分队。`collectible_mode_shopping` 表示凹开局是否购物。`collectible_mode_start_list` 是希望碰到的开局奖励，例如 `hot_water`。
- `monthly_squad_auto_iterate` 表示月度小队是否自动切换。`monthly_squad_check_comms` 表示切换时是否看通信。
- `deep_exploration_auto_iterate` 表示深入调查是否自动切换。
- `find_playTime_target` 是常乐节点。`1` 是令，`2` 是黍，`3` 是年。
- `blackflow_strategy` 是黑流树海的结局策略，例如 `ending1`。`blackflow_cultivation_target` 是要培育的襁褓动物。`blackflow_start_animals` 是准备局要凑齐的襁褓动物。
- `expected_collapsal_paradigms` 是希望碰到的坍缩范式。
- `first_floor_foldartal` 和 `start_foldartal_list` 是密文板。
- `start_with_seed` 是固定种子。

黑流树海的开局奖励位组合，以及快速飞行，没有写入。
图形界面里这两项不是关闭或「无」时，摘要里会各有一行跳过的字段。

### 生息演算

这些配置未经实机验证。
相关参数的作用见 [MAA 集成文档][task-types]。

- `theme` 是主题。
- `mode` 为 `0` 是不存档的繁荣，为 `1` 是写入存档的繁荣。认不出的模式不写，并记入摘要。
- `tools_to_craft` 是要制作的工具。
- `increment_mode` 是增量模式。
- `num_craft_batches` 是每轮制作批数。

仓库识别和干员 box 不会写成任务。
不认识的任务也不会写成任务。
这两类只出现在摘要里。

## 迁移摘要

有跳过或关掉的任务时，摘要写到标准错误。
没有任何损失时，什么都不打印。
命令打印的是英文。
下面三项不一定同时出现。

```text
Migration summary (lossy):
  Skipped tasks (unsupported type):
    - UserDataUpdateTask "同步数据"
  Disabled tasks (kept with params.enable=false; will not run):
    - FightTask "剿灭"
  Skipped fields (not migrated):
    - RecruitTask "公开招募": ForceRefresh
```

第一行表示这次转换丢掉了一部分内容。

`Skipped tasks` 是整项没有写进文件的任务。
引号里是图形界面里该任务的名字。
`UserDataUpdateTask` 是仓库识别和干员 box。

`Disabled tasks` 是已经写进文件、但默认不执行的任务。
括号里的 `params.enable=false` 就是上一节的 `enable`。
引号里是任务名。
`FightTask` 是作战，`RecruitTask` 是公开招募，`InfrastTask` 是基建。

`Skipped fields` 是某个任务里没写进去的一项设置。
引号里是任务名。
冒号后面是图形界面导出文件里的原字段名。
你不需要认识这个名字。
它只表示这项设置被丢掉了。
公开招募打开了立即刷新时，就会看到上面最后一行。

先看摘要，再把输出文件用于日常任务。

## 参考

- [配置](config.md)
- [使用](usage.md)
- [MAA 集成文档里的任务类型][task-types]

[task-types]: https://maa.plus/docs/zh-cn/protocol/integration.html#任务类型一览

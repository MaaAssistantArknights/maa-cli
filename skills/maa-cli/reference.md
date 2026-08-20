# maa-cli 参考

SKILL.md 的补充。只在需要物品 ID、OCR 边界或商店建议时再读。

## 物品 ID（俗称）

来自已安装的 MaaResource `item_index.json`（`maa dir resource`）。

|ID|名称|classifyType|备注|
|---|---|---|---|
|`4001`|龙门币|NORMAL||
|`4002`|至纯源石|NORMAL||
|`4003`|合成玉|NORMAL||
|`4004`|高级凭证|NORMAL|黄票|
|`4005`|资质凭证|NORMAL|绿票|
|`4006`|采购凭证|NORMAL|红票|
|`REP_COIN`|情报凭证|NORMAL||
|`3003`|赤金|NORMAL||
|`7001`|招聘许可|NORMAL||
|`7002`|加急许可|NORMAL||
|`7003`|寻访凭证|NORMAL||
|`7004`|十连寻访凭证|NORMAL||
|`classic_gacha`|中坚寻访凭证|NORMAL||
|`30012`|固源岩|MATERIAL||

`classifyType == MATERIAL` 才是 Depot 材料页模板白名单（芯片、精英材料、技能书、作战记录、模组数据块等）。票和货币是 `NORMAL`，不进该白名单。

## Depot / OperBox

两者都是 MaaCore 任务类型，maa-cli 只在 callback 里把 JSON 打到日志，**不落盘、没有 `maa query`。**
自定义任务示例：

```toml
[[tasks]]
type = "Depot"
```

Depot 是截图模板匹配 + 数量 OCR，不是读游戏内存，也不是「屏幕上有什么就扫什么」。

### Depot 实际覆盖

- 默认在仓库**材料**页滑动，只匹配 `MATERIAL` 模板。
- 较新的 MaaCore（自 v6.14.2 起部分版本）会再点「全部」，硬编码再扫：`4002` `4003` `4001` `3003` `4006`（源石、合成玉、龙门币、赤金、红票）。
- **默认不扫**：黄票 `4004`、绿票 `4005`、情报凭证、寻访/中坚寻访/十连、招聘许可、加急许可。这些多在消耗品或其它分页。
- 理智在战斗 callback `SanityBeforeStage`，不在 Depot。
- 信用点走 `Mall`，不是仓库。

### OperBox

- 有：`id` / `name` / `own` / `elite` / `level` / `potential` / `rarity`。
- 无：技能等级、专精、模组、信赖、基建技能解锁。

## Mall 与凭证店

`Mall` 只自动买**信用商店**，会真实点击购买。
红/绿/黄票店、情报凭证店：**没有**预设扫描或自动购买。

凭证店「库存」是**本号本周期限购余量**，不是全服抢货。
购买建议（只输出、不代买）需要：

1. catalog：货盘与价格（GameData `shop_client_table` 等，红绿票店货盘相对稳定）。
2. account：票余量（需扩 Depot 或商店顶栏 OCR，**现成命令没有**）。
3. account：材料缺口（现有 Depot 材料页）。
4. 限购余量：商店页 OCR 或自行记账（现成命令没有）。

不要声称已经根据黄/绿票余量给过精确可买清单，除非用户提供了这些数字或另有扫描结果。

## MaaResource vs GameData

MaaResource（`maa install` 装的）够 OCR 和作业，不够养成 wiki：

- 有：物品名、关卡掉落**种类**、干员职业/射程、基建技能**文案与排班效率**、公招 tags。
- 无：攻防 HP、战斗技能各等级数值、天赋、专精/模组材料与解锁条件。

完整表用 ArknightsGameData（`character_table` / `skill_table` / `building_data` / `item_table` / `stage_table` / `uniequip_table`），适合 ingest 成 SQLite 再查。
掉率用企鹅物流，不要写进 GameData。

## 常用 Fight 参数

- `-m/--medicine`：理智药数量。
- `--stone`：源石；`--dr-grandet` 等 1 理智再确认。
- `--times`：次数上限。
- `-D30012=100`：指定掉落件数后停止，可重复。
- `--series`：连战次数，`-1`～`6`。

日志：`MAA_LOG`，`-v`/`-q`，`--log-file`；任务总结可用 `--no-summary` 关掉。

## 自定义任务位置

- 任务：`$MAA_CONFIG_DIR/tasks/`
- 基建计划 JSON：`$MAA_CONFIG_DIR/infrast/`（必须 JSON，maa-cli 不按时段读计划文件，要用 `condition` + `plan_index`）
- 示例：`crates/maa-cli/config_examples/tasks/daily.json`
- Schema：`crates/maa-cli/schemas/task.schema.json`

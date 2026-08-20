# maa-cli 参考

任务类型和识别边界。旗标以 `maa help` 为准。

## Contents

- [子命令 vs 任务类型](#子命令-vs-任务类型)
- [Copilot URI](#copilot-uri)
- [客户端](#客户端)
- [物品 ID 与 Depot](#物品-id-与-depot)
- [OperBox / Mall](#operbox--mall)
- [安装资源里有什么](#安装资源里有什么)
- [条件与 batch](#条件与-batch)

## 子命令 vs 任务类型

|能力|有子命令|自定义任务 `type`|
|---|---|---|
|启动/关闭|`startup` / `closedown`|`StartUp` / `CloseDown`|
|作战|`fight`|`Fight`|
|作业|`copilot` / `ssscopilot` / `paradoxcopilot`|`Copilot` / `SSSCopilot` / `ParadoxCopilot`|
|肉鸽|`roguelike`|`Roguelike`|
|生息演算|`reclamation`|`Reclamation`|
|基建 / 公招 / 信用商店 / 领奖 / 仓库 / 干员箱|无|`Infrast` / `Recruit` / `Mall` / `Award` / `Depot` / `OperBox`|
|其它|无|`Custom` / `SingleStep` / `VideoRecognition`|

自定义任务文件：`$MAA_CONFIG_DIR/tasks/`，`maa run <stem>`。
基建计划 JSON：`$MAA_CONFIG_DIR/infrast/`（必须 JSON）。maa-cli 不按时段读计划文件，要用任务 `condition` 配 `plan_index`。
Schema：<https://github.com/MaaAssistantArknights/maa-cli/blob/main/crates/maa-cli/schemas/task.schema.json>
参数权威：[集成文档 · 任务类型](https://maa.plus/docs/zh-cn/protocol/integration.html)

`maa fight --help` 给出作战旗标。`--stone` 先确认；`-D<物品ID>=件数` 的 ID 来自 `item_index.json`（`maa dir resource`）。

未知 `connection.preset` 会被当成 `ADB` 并打警告。PlayCover 的别名是 `PlayTools`。

## Copilot URI

优先 `prts://<id>`（单份作业）或 `prts://s<id>`（作业集）。
也接受本地路径或 `file://`。
`maa://<id>` / `maa://<id>s` 仍能用，但会告警弃用。

## 客户端

|值|说明|PlayCover bundle|Android 包名|
|---|---|---|---|
|`Official`|官服|`com.hypergryph.arknights`|`com.hypergryph.arknights`|
|`Bilibili`|B 服|无|`com.hypergryph.arknights.bilibili`|
|`txwy`|台服|`tw.txwy.ios.arknights`|`tw.txwy.and.arknights`|
|`YoStarEN`|国际服|`com.YoStarEN.Arknights`|同左|
|`YoStarJP`|日服|`com.YoStarJP.Arknights`|同左|
|`YoStarKR`|韩服|`com.YoStarKR.Arknights`|同左|

## 物品 ID 与 Depot

票和货币的常用 ID（其余查 `item_index.json`）：

|ID|名称|classifyType|备注|
|---|---|---|---|
|`4001`|龙门币|NORMAL||
|`4002`|至纯源石|NORMAL||
|`4003`|合成玉|NORMAL||
|`4004`|高级凭证|NORMAL|黄票|
|`4005`|资质凭证|NORMAL|绿票|
|`4006`|采购凭证|NORMAL|红票|
|`REP_COIN`|情报凭证|NORMAL||
|`30012`|固源岩|MATERIAL|`-D` 示例常用|

`classifyType == MATERIAL` 才是 Depot 材料页模板白名单。票和货币是 `NORMAL`。

Depot 是自定义任务 `type = "Depot"`，截图模板匹配 + 数量 OCR。结果打在日志里，不落盘。
部分 Core 会再扫「全部」页的源石/合成玉/龙门币/赤金/红票；黄票、绿票、情报凭证、寻访凭证、招聘许可以当前日志为准。
理智在战斗 callback `SanityBeforeStage`。信用点走 `Mall`。

## OperBox / Mall

OperBox 日志字段：`id` / `name` / `own` / `elite` / `level` / `potential` / `rarity`。技能等级、专精、模组、信赖不在结果里。

`Mall` 只自动买信用商店，会真实点击购买。红/绿/黄票店和情报凭证店没有扫描或自动购买子命令。没有 `maa snapshot` / `maa query`。
凭证店「库存」是本号本周期限购余量。没扫到的票余量和限购次数不当成已经读到。
中期方向（OCR 扩采集 + 按需查询，决策是消费者）见 [信息查询面](../../docs/design/query-surface.md)。

## 安装资源里有什么

`maa install` 装的资源够识别图标和跑作业：物品名、关卡掉落种类、干员职业/射程、基建技能文案与排班效率、公招 tags。
养成数值（攻防 HP、技能各等级、专精/模组材料）查游戏数据或 wiki。掉率查企鹅物流。

## 条件与 batch

自定义任务可用 `Time` / `DateTime` / `Weekday` / `DayMod` / `OnSideStory` 以及 `And`/`Or`/`Not`。
`timezone = "Official"` 用游戏日界（官服 UTC+4），不是东八区 0 点。
当天 `DayMod` 余数：`maa remainder <divisor>`。

任务参数里的 `Input`/`Select` 在交互运行时会提问。`--batch` 用默认值；没有默认值就失败。

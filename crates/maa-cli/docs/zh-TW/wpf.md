# 從 WPF GUI 遷移設定

`maa migrate wpf` 把 MAA Windows 圖形介面匯出的 JSON 轉成 maa-cli 任務檔。
不支援的任務和設定會被跳過。
結束時，命令把摘要寫到標準錯誤。
轉完後對照 [設定](config.md) 檢查一遍，再用於日常任務。

## 遷移一份設定

輸入是圖形介面匯出的 JSON 檔。

```bash
# 預設寫到與輸入同名的 .toml
maa migrate wpf gui.json

# 副檔名決定格式。可以是 toml、yaml 或 json
maa migrate wpf gui.json tasks/daily.toml

# 匯出裡有多份設定時，用名字選一份
maa migrate wpf gui.json --profile-name Default

# 圖形介面裡已經是自訂排班時，改用你給出的排班檔
maa migrate wpf gui.json --custom-schedule ~/base-schedule.json
```

匯出裡只有一份設定時，命令直接用它。
有多份，並且沒有 `--profile-name` 時，命令會讓你選。

`--custom-schedule` 只在圖形介面裡這份基建已經是自訂排班時有效。
還不是自訂排班時，加上這個選項會報錯退出。

把生成的檔案放到 `$MAA_CONFIG_DIR/tasks`，再用 `maa run <task>` 執行。
設定目錄用 `maa dir config` 查看。

## 設定文件裡已經有的設定

輸出和 [自訂任務](config.md#自定义任务) 的結構相同。
頂層是 `[[tasks]]`。
每個任務有 `type`、`name` 和 `params`。
需要按條件切換時還有 `variants`。
條件的寫法見 [任務條件](config.md#任务条件)。

下面只說明設定文件裡已經出現過的名字。
設定文件沒寫、執行時直接交給 MaaCore 的名字，在下一節。

### 開始喚醒

`client_type` 和 `start_game_enabled` 就是設定文件啟動示例裡的那兩個。
圖形介面沒有可識別的客戶端時，`client_type` 會變成執行時再選的輸入。
寫法見設定文件的使用者輸入。
開了切換帳號但沒有帳號名時，這項不寫入。
摘要裡會有一行跳過的欄位。

### 理智作戰

每個要打的關卡是一個變體。
關卡名寫在 `stage`。

條件按關卡種類寫。

- 資源本或晶片本用 `Weekday`，時區是官服。
- 活動關能在本機熱更新的活動表裡查到時，用 `DateTime`。裡面有開始時間、結束時間和時區。查不到則用 `Always`，並告警。
- 主線、剿滅和 `LS-6` 這類常駐關用 `Always`。

`DateTime`、`Weekday` 和 `DayMod` 在設定文件的任務條件裡。
`Always` 也是 maa-cli 的條件，表示不看時間，這個變體總是可以執行。
活動關的 `start` 和 `end` 取自遷移當時本機的活動表。
活動換檔後不會自動更新。
需要重新遷移，或手改條件。
圖形介面裡還有一份每週計畫時，週計畫和關卡條件用 `And` 合在一起。

下面是多個備選關卡時的結果。

```toml
[[tasks]]
name = "理智作戰"
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

如果結果裡會使用源石，命令會警告。
這個設定可能消耗源石。
理智藥、源石、次數和過期藥的參數名不在設定文件裡，見下一節。

### 基建換班

設施順序寫在 `facility`。
宿舍是否填入信賴未滿的幹員寫在 `dorm_trust_enabled`。
這兩個名字在設定文件的基建例子裡。

圖形介面按時間自動換班，並且排班裡每個班次都有時段時，`mode` 是 `10000`。
變體按時間段選擇 `plan_index`。
跨天時段會加上 `DayMod`。
這和設定文件裡的基建例子是同一套。
排班沒有可用時段，或圖形介面選了固定班次時，結果仍是自訂基建，並帶上 `plan_index`。

寫出的 `filename` 是絕對路徑。
排班路徑裡的 `~` 展開成你的主目錄。
相對路徑按你傳給命令的那份 JSON 所在目錄解析。
設定文件說明，這份排班 JSON 由 MaaCore 讀取，不由 maa-cli 讀取。
排班檔裡的菲亞梅塔也留在那個檔案裡，不抄進任務參數。

要換成另一份排班，用 `--custom-schedule` 給出那份 JSON 的路徑。
圖形介面還不是自訂排班時，命令報錯退出。
指定的檔案讀失敗時也報錯退出，不會退回預設基建。

沒有這個選項，並且排班不存在、無法解析或沒有班次時，`mode` 寫成 `0`。
摘要裡會說明跳過了排班。

無人機、心情閾值、菲亞梅塔名單這些名字不在設定文件裡，見下一節。

## 交給 MaaCore 的參數

[設定](config.md#基本结构) 寫明，任務 `params` 裡的具體名字以 MAA 整合文件為準。
maa-cli 不檢查這些名字對不對。
執行時由 MaaCore 讀取。
下面這些就是遷移會寫進 `params`、但設定文件沒有解釋的名字。
要改數值，改這些名字。
每個名字的全部可選取值見 [MAA 整合文件][task-types]。

### 是否執行

`enable` 為 `false` 時，這個任務不執行。
圖形介面裡關掉的任務仍會寫進檔案，並帶上 `enable = false`。
要讓它跑，把該欄位改成 `true`，或刪掉它。

### 理智作戰參數

- `medicine` 是理智藥個數。設定文件只在使用者輸入的例子裡出現過這個名字，沒有說明它是理智藥。
- `stone` 是源石個數。
- `times` 是次數上限。
- `series` 是連戰次數。
- `drops` 是目標掉落。
- `medicine_expire_days` 是使用多少天內將過期的理智藥。設定文件示例裡的 `expiring_medicine` 是另一個名字，遷移不會寫成那個名字。
- `DrGrandet` 為 `true` 時，在使用源石的確認介面等到回復 1 點理智，再立刻使用。

### 公開招募

- `times` 是招募次數。
- `refresh` 表示是否重新整理三星標籤。
- `extra_tags_mode` 是額外標籤的處理方式。數字的含義見整合文件。
- `select` 和 `confirm` 是要自動選擇、自動確認的星級，例如 `3`、`4`、`5`。
- `recruitment_time` 是各星級的招募分鐘數。
- `first_tags` 是優先標籤。
- `preserve_tags` 是要保留的標籤。
- `level3_recruitment_permit_reserve` 是三星招募許可要保留的數量。這項沒開，或數量不大於 0 時，不寫這個名字。

圖形介面裡的立即重新整理不會寫入。
它開著時，摘要裡會有一行跳過的欄位，並警告。
開了自動確認六星時，命令會警告。

### 基建換班參數

- `drones` 是無人機用途。
- `threshold` 是宿舍心情閾值，範圍是 0 到 1。圖形介面裡的百分比會換成這個小數。
- `replenish` 表示貿易站的源石碎片是否自動補貨。
- `dorm_notstationed_enabled` 表示宿舍是否啟用「未進駐」。
- `reception_message_board`、`reception_clue_exchange` 和 `reception_send_clue` 是會客室的留言板、線索交流和送出線索。
- `continue_training` 表示訓練室是否繼續訓練。
- `fiammetta_targets` 是菲亞梅塔要換心情的幹員名單。
- `fiammetta_recovery_enabled` 表示是否用菲亞梅塔恢復心情。
- `use_pinus_sylvestris`、`use_perception_information`、`use_worldly_plight` 和 `use_abyssal_hunter` 是紅松、感知、塵世和深海這四組跨設施組合。為 `false` 時不寫。

`fiammetta_targets` 只在 `mode` 為 0 且 `fiammetta_recovery_enabled` 為 true 時生效。
`fiammetta_recovery_enabled` 和上面四組組合只在 `mode` 為 0 時生效。

### 信用商店

- `shopping` 表示是否購買信用商品。
- `credit_fight` 表示是否進行信用作戰。
- `formation_index` 是信用作戰使用的編隊。
- `visit_friends` 表示是否訪問好友。
- `buy_first` 是優先購買的商品。
- `blacklist` 是不購買的商品。
- `force_shopping_if_credit_full` 表示信用滿了是否仍按黑名單以外的商品購買。
- `only_buy_discount` 表示是否只買折扣商品。
- `reserve_max_credit` 表示是否把信用留到上限附近。

信用作戰和訪問好友的上次時間、今天是否還能做，是圖形介面自己的記錄。
這些不寫入結果。

### 領獎

- `award` 是領取每日或每週任務獎勵。
- `mail` 是領取郵件獎勵。
- `recruit` 是領取限定池贈送的每日免費單抽。
- `orundum` 是領取幸運牆的合成玉。
- `mining` 是領取限時開採許可的合成玉。
- `specialaccess` 是領取五週年贈送的月卡。
- `signinevent` 是領取限時簽到獎勵。

### 肉鴿

這些設定未經實機驗證。
相關參數的作用見 [MAA 整合文件][task-types]。

這些名字只在對應的主題和模式裡有用。
寫進去是為了留下你在圖形介面裡選過的內容。
MaaCore 在用不到的模式下會忽略它們。

- `theme` 是主題。
- `mode` 是模式編號。刷分是 `0`，刷源石錠是 `1`，凹開局是 `4`，坍縮範式是 `5`，月度小隊是 `6`，深入調查是 `7`，刷常樂節點是 `20001`，黑流樹海刷開局是 `30003`。認不出的模式不寫 `mode`，並記入摘要。
- `squad` 是開局分隊。`roles` 是開局職業組合。
- `core_char` 是第一名開局幹員。`core_char_list` 是全部開局幹員，每項有 `name` 和 `use_support`。
- `use_support` 表示第一名是否使用助戰。`use_nonfriend_support` 表示是否使用非好友助戰。
- `starts_count` 是開始次數。`difficulty` 是難度。難度如果是一個佔位用的極大數字，則不寫。
- `investment_enabled` 表示是否投資。`investments_count` 是投資次數。`investment_with_more_score` 表示是否用更高分數投資。`stop_when_investment_full` 表示存滿後是否停止。
- `stop_at_final_boss` 表示是否在結局前停止。`stop_at_max_level` 表示等級滿了是否停止。
- `start_with_elite_two` 和 `only_start_with_elite_two` 是精二開局。
- `refresh_trader_with_dice` 表示是否用骰子重新整理商店。
- `collectible_mode_squad` 是凹開局或黑流樹海準備局使用的分隊。`collectible_mode_shopping` 表示凹開局是否購物。`collectible_mode_start_list` 是希望碰到的開局獎勵，例如 `hot_water`。
- `monthly_squad_auto_iterate` 表示月度小隊是否自動切換。`monthly_squad_check_comms` 表示切換時是否看通信。
- `deep_exploration_auto_iterate` 表示深入調查是否自動切換。
- `find_playTime_target` 是常樂節點。`1` 是令，`2` 是黍，`3` 是年。
- `blackflow_strategy` 是黑流樹海的結局策略，例如 `ending1`。`blackflow_cultivation_target` 是要培育的襁褓動物。`blackflow_start_animals` 是準備局要湊齊的襁褓動物。
- `expected_collapsal_paradigms` 是希望碰到的坍縮範式。
- `first_floor_foldartal` 和 `start_foldartal_list` 是密文板。
- `start_with_seed` 是固定種子。

黑流樹海的開局獎勵位組合，以及快速飛行，沒有寫入。
圖形介面裡這兩項不是關閉或「無」時，摘要裡會各有一行跳過的欄位。

### 生息演算

這些設定未經實機驗證。
相關參數的作用見 [MAA 整合文件][task-types]。

- `theme` 是主題。
- `mode` 為 `0` 是不存檔的繁榮，為 `1` 是寫入存檔的繁榮。認不出的模式不寫，並記入摘要。
- `tools_to_craft` 是要製作的工具。
- `increment_mode` 是增量模式。
- `num_craft_batches` 是每輪製作批數。

倉庫識別和幹員 box 不會寫成任務。
不認識的任務也不會寫成任務。
這兩類只出現在摘要裡。

## 遷移摘要

有跳過或關掉的任務時，摘要寫到標準錯誤。
沒有任何損失時，什麼都不列印。
命令列印的是英文。
下面三項不一定同時出現。

```text
Migration summary (lossy):
  Skipped tasks (unsupported type):
    - UserDataUpdateTask "同步数据"
  Disabled tasks (kept with params.enable=false; will not run):
    - FightTask "剿灭"
  Skipped fields (not migrated):
    - RecruitTask "公开招募": ForceRefresh
```

第一行表示這次轉換丟掉了一部分內容。

`Skipped tasks` 是整項沒有寫進檔案的任務。
引號裡是圖形介面裡該任務的名字。
`UserDataUpdateTask` 是倉庫識別和幹員 box。

`Disabled tasks` 是已經寫進檔案、但預設不執行的任務。
括號裡的 `params.enable=false` 就是上一節的 `enable`。
引號裡是任務名。
`FightTask` 是作戰，`RecruitTask` 是公開招募，`InfrastTask` 是基建。

`Skipped fields` 是某個任務裡沒寫進去的一項設定。
引號裡是任務名。
冒號後面是圖形介面匯出檔裡的原欄位名。
你不需要認識這個名字。
它只表示這項設定被丟掉了。
公開招募打開了立即重新整理時，就會看到上面最後一行。

先看摘要，再把輸出檔用於日常任務。

## 參考

- [設定](config.md)
- [使用](usage.md)
- [MAA 整合文件裡的任務類型][task-types]

[task-types]: https://maa.plus/docs/zh-tw/protocol/integration.html

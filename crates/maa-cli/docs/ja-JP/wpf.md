# WPF GUI から設定を移す

`maa migrate wpf` は、MAA の Windows GUI が書き出した JSON を maa-cli のタスクファイルに変換します。
対応していないタスクと設定は飛ばします。
終了時、コマンドは要約を標準エラーへ書きます。
日常の実行に使う前に、結果を [設定](config.md) と照合してください。

## 設定を 1 つ移す

入力は GUI が書き出した JSON ファイルです。

```bash
# 入力と同じ名前の .toml を書く
maa migrate wpf gui.json

# 拡張子が形式を決める。toml、yaml、json のいずれか
maa migrate wpf gui.json tasks/daily.toml

# 書き出しに設定が複数あるときは、名前で 1 つ選ぶ
maa migrate wpf gui.json --profile-name Default

# GUI がすでにカスタム基建スケジュールのとき、指定したスケジュールファイルを使う
maa migrate wpf gui.json --custom-schedule ~/base-schedule.json
```

書き出しに設定が 1 つだけのとき、コマンドはそれを使います。
複数あり、`--profile-name` がないとき、コマンドは選択を尋ねます。

`--custom-schedule` は、その基建が GUI 上ですでにカスタムスケジュールのときだけ有効です。
まだカスタムスケジュールでないとき、このオプションを付けるとコマンドはエラーで終了します。

生成したファイルを `$MAA_CONFIG_DIR/tasks` に置き、`maa run <task>` で実行します。
設定ディレクトリは `maa dir config` で確認します。

## 設定ドキュメントに既にある項目

出力の形は [カスタムタスク](config.md#自定义任务) と同じです。
最上段は `[[tasks]]` です。
各タスクには `type`、`name`、`params` があります。
条件で切り替えるときは `variants` もあります。
条件の書き方は [タスク条件](config.md#任务条件) を参照してください。

ここから先で挙げる名前は、設定ドキュメントに既にあるものだけです。
設定ドキュメントに無く、実行時に MaaCore が読む名前は次の節です。

### 起動

`client_type` と `start_game_enabled` は、設定ドキュメントの起動例にある 2 つの項目です。
GUI に判別できるクライアントがないとき、`client_type` は実行時に選ぶ入力になります。
書き方は設定ドキュメントのユーザー入力を参照してください。
アカウント切り替えが有効でアカウント名が空のとき、その項目は書き込まれません。
要約に、飛ばしたフィールドが 1 行出ます。

### 理性作戦

打つステージはそれぞれ 1 つのバリアントです。
ステージ名は `stage` です。

条件はステージの種類に従います。

- 資源ステージまたはチップステージは、公式サーバーのタイムゾーンで `Weekday` になります。
- イベントステージは、このマシンのホットアップデートにあるイベント表にそのイベントがあるとき `DateTime` になります。開始時刻、終了時刻、タイムゾーンが入ります。無いときは `Always` になり、警告が出ます。
- メインストーリー、殲滅、`LS-6` のような常設ステージは `Always` です。

`DateTime`、`Weekday`、`DayMod` は設定ドキュメントのタスク条件にあります。
`Always` も maa-cli の条件です。時刻を見ないので、そのバリアントは常に実行できます。
イベントステージの `start` と `end` は、移行した時点のこのマシン上のイベント表から取ります。
その後のホットアップデートには追従しません。
イベントが変わったら、再度移行するか、条件を手で直します。
GUI に週間予定もあるとき、週間予定とステージ条件は `And` でまとまります。

次の例は、候補ステージが複数あるときの結果です。

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

結果が純正源石を使うとき、コマンドは警告します。
この設定は純正源石を消費することがあります。
理性薬、純正源石、回数、期限切れ理性薬のパラメータ名は設定ドキュメントにありません。次の節を参照してください。

### 基建の交代

施設の順序は `facility` です。
信頼度が最大でないオペレーターを宿舎に入れるかは `dorm_trust_enabled` です。
この 2 つの名前は、設定ドキュメントの基建の例にあります。

GUI が時刻で交代し、スケジュールの各シフトに時間帯があるとき、`mode` は `10000` です。
バリアントは時間帯で `plan_index` を選びます。
日付をまたぐ時間帯には `DayMod` が付きます。
これは設定ドキュメントの基建の例と同じ組み立てです。
スケジュールに使える時間帯がないとき、または GUI が固定シフトを選んでいるとき、結果はカスタム基建のまま `plan_index` を含みます。

書き出される `filename` は絶対パスです。
スケジュールパスの `~` はホームディレクトリに展開されます。
相対パスは、コマンドに渡した JSON があるディレクトリから解決されます。
設定ドキュメントにあるとおり、このスケジュール JSON を読むのは maa-cli ではなく MaaCore です。
スケジュールファイル内のフィアメッタもそのファイルに残り、タスクパラメータには写しません。

別のスケジュールを使うには、`--custom-schedule` でその JSON のパスを渡します。
GUI がまだカスタムスケジュールでないとき、コマンドはエラーで終了します。
指定したファイルを読めないときもエラーで終了し、既定の基建には戻りません。

このオプションが無く、スケジュールが無い、解析できない、またはシフトが無いとき、`mode` は `0` と書かれます。
要約には、スケジュールを飛ばしたと出ます。

ドローン、気分のしきい値、フィアメッタの名簿は、設定ドキュメントに無い名前です。次の節を参照してください。

## MaaCore に渡すパラメータ

[基本構造](config.md#基本结构) にあるとおり、タスク `params` の具体的な名前は MAA 統合ドキュメントに従います。
maa-cli はその名前が正しいかを検査しません。
実行時に MaaCore が読みます。
以下は、移行が `params` に書くが、設定ドキュメントが説明していない名前です。
値を変えるときは、これらの名前を変えます。
各名前の取りうる値は [MAA 統合ドキュメント][task-types] を参照してください。

### 実行するかどうか

`enable` が `false` のとき、そのタスクは実行されません。
GUI でオフのタスクもファイルには残り、`enable = false` が付きます。
実行するには、そのフィールドを `true` にするか、削除します。

### 理性作戦のパラメータ

- `medicine` は理性薬の個数です。設定ドキュメントでは、ユーザー入力の例にこの名前が出るだけで、理性薬だとは書いてありません。
- `stone` は純正源石の個数です。
- `times` は回数の上限です。
- `series` は連戦の回数です。
- `drops` は目標ドロップです。
- `medicine_expire_days` は、期限まで何日以内の理性薬を使うかです。設定ドキュメントの例にある `expiring_medicine` は別の名前で、移行はそちらを書きません。
- `DrGrandet` が `true` のとき、純正源石の確認画面で理性が 1 回復するまで待ち、すぐに使用します。

### 公開求人

- `times` は求人の回数です。
- `refresh` は星 3 タグを更新するかどうかです。
- `extra_tags_mode` は追加タグの扱いです。数値の意味は統合ドキュメントを参照してください。
- `select` と `confirm` は、自動で選び、自動で確定する星の数です。例は `3`、`4`、`5` です。
- `recruitment_time` は星ごとの求人分数です。
- `first_tags` は優先タグです。
- `preserve_tags` は残すタグです。
- `level3_recruitment_permit_reserve` は残しておく星 3 求人許可証の数です。この項目がオフ、または個数が 0 より大きくないとき、この名前は書きません。

GUI の即時更新は書き込まれません。
オンのとき、要約に飛ばしたフィールドが 1 行出て、警告も出ます。
星 6 の自動確定がオンのとき、コマンドは警告します。

### 基建交代のパラメータ

- `drones` はドローンの用途です。
- `threshold` は宿舎の気分しきい値で、範囲は 0 から 1 です。GUI の百分率はこの小数になります。
- `replenish` は貿易所の源石の欠片を自動補充するかどうかです。
- `dorm_notstationed_enabled` は宿舎で「未配置」を使うかどうかです。
- `reception_message_board`、`reception_clue_exchange`、`reception_send_clue` は応接室の伝言板、手がかり交換、手がかりの送付です。
- `continue_training` は訓練室で訓練を続けるかどうかです。
- `fiammetta_targets` は、フィアメッタが気分を回復するオペレーターの名簿です。
- `fiammetta_recovery_enabled` は、フィアメッタで気分を回復するかどうかです。
- `use_pinus_sylvestris`、`use_perception_information`、`use_worldly_plight`、`use_abyssal_hunter` は、レッドパイン、知覚、塵世、アビサルの 4 つの施設組み合わせです。`false` のときは書きません。

`fiammetta_targets` が効くのは、`mode` が 0 かつ `fiammetta_recovery_enabled` が true のときだけです。
`fiammetta_recovery_enabled` と上の 4 つの組み合わせが効くのは、`mode` が 0 のときだけです。

### 信用交換

- `shopping` は信用商品を買うかどうかです。
- `credit_fight` は信用戦闘を行うかどうかです。
- `formation_index` は信用戦闘で使う編成です。
- `visit_friends` はフレンドを訪問するかどうかです。
- `buy_first` は優先して買う商品です。
- `blacklist` は買わない商品です。
- `force_shopping_if_credit_full` は、信用が満杯のときブラックリスト以外をまだ買うかどうかです。
- `only_buy_discount` は割引商品だけ買うかどうかです。
- `reserve_max_credit` は信用を上限付近まで残すかどうかです。

信用戦闘とフレンド訪問の前回時刻、今日まだできるかは、GUI 自身の記録です。
結果には書きません。

### 報酬

- `award` はデイリーまたはウィークリー任務の報酬を受け取ります。
- `mail` はメール報酬を受け取ります。
- `recruit` は限定ガチャの毎日の無料 1 回を受け取ります。
- `orundum` はラッキーウォールの合成玉を受け取ります。
- `mining` は期間限定の採掘許可証の合成玉を受け取ります。
- `specialaccess` は 5 周年でもらえる月間パスを受け取ります。
- `signinevent` は期間限定のサインイン報酬を受け取ります。

### ローグライク

これらの設定は実機では確認していません。
各パラメータの働きは [MAA 統合ドキュメント][task-types] を参照してください。

これらの名前が意味を持つのは、対応するテーマとモードだけです。
GUI で選んだ内容を残すために書き込みます。
使わないモードでは、MaaCore はそれらを無視します。

- `theme` はテーマです。
- `mode` はモード番号です。スコア周回は `0`、源石錠周回は `1`、開始報酬狙いは `4`、崩壊パラダイムは `5`、月次分隊は `6`、深層調査は `7`、常楽ノード狙いは `20001`、黒流樹海の開始周回は `30003` です。判別できないモードは `mode` を書かず、要約に記録します。
- `squad` は開始分隊です。`roles` は開始の職分の組み合わせです。
- `core_char` は最初の開始オペレーターです。`core_char_list` は開始オペレーター全員で、各項目に `name` と `use_support` があります。
- `use_support` は最初の 1 人をサポートにするかどうかです。`use_nonfriend_support` はフレンド以外のサポートを使うかどうかです。
- `starts_count` は開始回数です。`difficulty` は難易度です。難易度がプレースホルダの非常に大きい数なら書きません。
- `investment_enabled` は投資するかどうかです。`investments_count` は投資回数です。`investment_with_more_score` はより高いスコアで投資するかどうかです。`stop_when_investment_full` は預金が満杯なら止めるかどうかです。
- `stop_at_final_boss` は結末の前で止めるかどうかです。`stop_at_max_level` はレベルが最大なら止めるかどうかです。
- `start_with_elite_two` と `only_start_with_elite_two` はエリート 2 での開始です。
- `refresh_trader_with_dice` はサイコロで商店を更新するかどうかです。
- `collectible_mode_squad` は開始報酬狙い、または黒流樹海の準備局で使う分隊です。`collectible_mode_shopping` は開始報酬狙いで買い物するかどうかです。`collectible_mode_start_list` は欲しい開始報酬で、例は `hot_water` です。
- `monthly_squad_auto_iterate` は月次分隊を自動で切り替えるかどうかです。`monthly_squad_check_comms` は切り替え時に通信を見るかどうかです。
- `deep_exploration_auto_iterate` は深層調査を自動で切り替えるかどうかです。
- `find_playTime_target` は常楽ノードです。`1` はリィン、`2` はシュー、`3` はニェンです。
- `blackflow_strategy` は黒流樹海の結末方針で、例は `ending1` です。`blackflow_cultivation_target` は育てる襁褓の動物です。`blackflow_start_animals` は準備局で揃える襁褓の動物です。
- `expected_collapsal_paradigms` は欲しい崩壊パラダイムです。
- `first_floor_foldartal` と `start_foldartal_list` は密文板です。
- `start_with_seed` は固定シードです。

黒流樹海の開始報酬のビット組み合わせと、快速飛行は書き込まれません。
GUI でこの 2 つがオフでも「なし」でもないとき、要約に飛ばしたフィールドがそれぞれ 1 行出ます。

### 生息演算

これらの設定は実機では確認していません。
各パラメータの働きは [MAA 統合ドキュメント][task-types] を参照してください。

- `theme` はテーマです。
- `mode` が `0` ならセーブしない繁栄、`1` ならセーブへ書く繁栄です。判別できないモードは書かず、要約に記録します。
- `tools_to_craft` は作る道具です。
- `increment_mode` は増分モードです。
- `num_craft_batches` は 1 周あたりの製作バッチ数です。

倉庫認識とオペレーター box はタスクになりません。
未知のタスクもタスクになりません。
この 2 つは要約にだけ出ます。

## 要約

飛ばした項目やオフのタスクがあるとき、要約は標準エラーへ書かれます。
何も失っていないとき、何も印刷しません。
コマンドが印刷するのは英語です。
次の 3 項目は、必ず同時に出るわけではありません。

```text
Migration summary (lossy):
  Skipped tasks (unsupported type):
    - UserDataUpdateTask "同步数据"
  Disabled tasks (kept with params.enable=false; will not run):
    - FightTask "剿灭"
  Skipped fields (not migrated):
    - RecruitTask "公开招募": ForceRefresh
```

1 行目は、この変換が一部を落としたことを表します。

`Skipped tasks` は、ファイルにまったく書かれなかったタスクです。
引用符の中は、GUI でのそのタスクの名前です。
`UserDataUpdateTask` は倉庫認識とオペレーター box です。

`Disabled tasks` は、ファイルには書いたが、既定では実行しないタスクです。
括弧内の `params.enable=false` が、前の節の `enable` です。
引用符の中はタスク名です。
`FightTask` は作戦、`RecruitTask` は公開求人、`InfrastTask` は基建です。

`Skipped fields` は、あるタスクの中で書き込まれなかった設定 1 つです。
引用符の中はタスク名です。
コロンの後は、GUI の書き出しファイルにあった元のフィールド名です。
その名前を知る必要はありません。
その設定が落とされたことだけを表します。
公開求人で即時更新がオンのとき、上の最後の行になります。

日常の実行に使う前に、要約を読んでください。

## 参考

- [設定](config.md)
- [使い方](usage.md)
- [MAA 統合ドキュメントのタスクタイプ][task-types]

[task-types]: https://maa.plus/docs/ja-jp/protocol/integration.html

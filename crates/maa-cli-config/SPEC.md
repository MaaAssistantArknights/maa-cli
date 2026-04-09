# Config V2 设计规范

## 概述

- **Profile**（`profiles/*.toml`）：定义*如何*连接——连接方式、客户端与可选的行为/进阶配置
- **Task**（`tasks/*.yaml`）：定义*做什么*——带条件和覆盖的任务列表

Profile 更扁平，适合 TOML；Task 嵌套较深，适合 YAML。两种格式只是推荐，TOML、YAML、JSON 均受支持，可以混用。

### 设计目标

V2 的目标有两个：

- **对用户更直观**：Profile 负责描述"如何连接"，Task 负责描述"做什么"，同时不再直接对接 MaaCore API。`client_type` 等运行上下文不再散落在任务参数中；任务文件也不再依赖多种隐式变换和不一致的规则，尤其体现在生命周期处理、参数传播，以及 variants 的激活逻辑上。
- **对实现更直接**：配置边界更清晰，`client_type` 的来源唯一，自动生命周期与手写生命周期的语义分离，单 Session 与多 Session 也分别建模。这样可以避免在运行时反复提取、合并、回填和兜底，减少隐式规则和特殊分支。

### 兼容性

Profile 和 Task 都有一个顶层字段 `version`：

```toml
version = 2
```

- `version = 2` → 按 V2 解析
- `version = 1` → 按 V1 解析，不警告
- 无 `version` → 按 V1 解析 + 弃用警告

Profile 和 Task 必须同时升级，不保证交叉兼容，必须是 Profile V1 + Task V1 或者 Profile V2 + Task V2。

---

## Profile 配置

### 顶层字段

```toml
version = 2              # 必填，标识配置格式版本
inherits = "default"     # 可选，继承另一个 Profile，深度合并（子字段覆盖父字段）
client_type = "Official" # 可选
```

#### 继承示例

`inherits` 字段用来继承另一个 Profile，方便实现配置复用：

```toml
# profiles/default.toml
version = 2

[connection]
type = "General"
address = "emulator-5554"
touch_mode = "MaaTouch"
```

```toml
# profiles/yostar-en.toml
version = 2
inherits = "default"
client_type = "YoStarEN"   # 只改这一个字段，其余全部继承
```

#### 游戏客户端类型

`client_type` 是一个横跨多个作用域的配置，决定：

- 加载哪个全局资源包（如 YoStarEN → EN 资源）
- 部分 Preset 使用的游戏包名 / Bundle ID（如 PlayCover）
- 任务条件判断需要的 client_type
- 部分任务需要注入的 client_type

### `[connection]`

`type` 选择连接类型，决定哪些字段合法，以及是否需要管理外部环境（例如 Waydroid）。

```toml
[connection]
type = "General"               # General / PlayCover / Waydroid / AVD / MuMuPro
```

#### `General`（通用模式，直接对标原始的 MaaCore API）

类似于旧版本，但部分选项的位置和名字发生了变化。

```toml
[connection]
type = "General"
address = "emulator-5554"   # 可选，缺省时从 `adb devices` 自动检测
adb_path = "adb"            # 可选
touch_mode = "MaaTouch"     # 可选：MaaTouch / MiniTouch / Adb
adb_lite = false            # 可选
kill_adb_on_exit = false    # 可选
config = "General"          # 可选，连接时使用的配置，可能影响部分行为
```

#### `PlayCover`（macOS 通过 PlayCover 原生运行 iOS App）

`touch_mode` 内部固定为 `MacPlayTools`，平台差异资源 `iOS` 自动加载。

```toml
[connection]
type = "PlayCover"
address = "127.0.0.1:1717"  # 可选，默认 127.0.0.1:1717
screencap_mode = "Default"  # 可选：Default / BGR / SCK
```

截图模式说明：

- `Default`：默认兼容模式
- `BGR`：速度更快，目前没发现兼容性问题
- `SCK`：使用 macOS ScreenCaptureKit，速度最快，但需要终端宿主进程被授予截图权限

#### `Waydroid`（Linux Container）

支持自动启动，连接地址在运行时从 `waydroid status` 获取：

```toml
[connection]
type = "Waydroid"
adb_path = "adb"           # 可选
touch_mode = "MaaTouch"    # 可选
adb_lite = false           # 可选
```

#### `MuMuPro`（macOS）

类似于通用 ADB，但有已知的 adb_path 和 address：

```toml
[connection]
type = "MuMuPro"
address = "127.0.0.1:16384"  # 可选
touch_mode = "MaaTouch"      # 可选
adb_lite = false             # 可选
kill_adb_on_exit = false     # 可选
```

#### `AVD`（Android 虚拟设备）

`sdk_path` 必填，`adb_path` 和模拟器命令从中推导：

```toml
[connection]
type = "AVD"
sdk_path = "/home/user/Android/Sdk"   # 必填
avd_name = "Pixel_6_API_33"           # 可选，用于启动指定模拟器
touch_mode = "MaaTouch"               # 可选
adb_lite = false                      # 可选
kill_adb_on_exit = false              # 可选
```

### `[behavior]`

运行时行为偏好，所有字段可选。

```toml
[behavior]
auto_reconnect = true          # 掉线后自动重连，默认 true
deployment_with_pause = false  # 部署时暂停，默认 false
```

### `[advanced]`

进阶技术配置，大多数用户无需关注。所有字段可选。

```toml
[advanced]
inference_engine = "cpu"       # cpu / gpu:0 / gpu:1 / ...，默认 cpu
user_resource = false          # 从配置目录加载自定义资源，默认 false
```

### 完整示例

**最简配置（PlayCover）：**

```toml
version = 2

[connection]
type = "PlayCover"
```

**完整配置（General）：**

```toml
version = 2
client_type = "YoStarEN"

[connection]
type = "General"
address = "127.0.0.1:5555"
adb_path = "/usr/bin/adb"
touch_mode = "MaaTouch"
adb_lite = false
kill_adb_on_exit = false

[behavior]
auto_reconnect = true

[advanced]
inference_engine = "gpu:0"
user_resource = true
```

---

## Task 配置（`tasks/*.yaml`）

### 生命周期管理

两个标志控制自动启停：

- `manage_environment_lifecycle`（默认 `true`）：是否自动管理外部环境（如 Waydroid session、AVD）。由 Profile 的 `connection.type` 决定是否实际生效——General / PlayCover 等无外部环境的类型忽略此标志。
- `manage_game_lifecycle`（默认 `true`）：是否自动管理游戏启停流程。

**自动模式**（`manage_game_lifecycle = true`）：

- 在任务列表前后自动执行 StartUp / CloseDown
- `account_name` 会自动注入给 StartUp，`client_type` 会自动注入给需要的任务参数
- CloseDown 在以下情况自动执行：正常结束、运行报错、掉线停止
- 用户主动中断（Ctrl-C）时不执行 CloseDown

**手写模式**（`manage_game_lifecycle = false`）：

- `tasks` 中可以手写 StartUp / CloseDown，并允许任意穿插
- 运行时严格按书写顺序执行，不做任何额外兜底
- `client_type` 仍由 Profile 注入给 StartUp，但不会从 task params 中提取

### 任务

Task 文件支持两种模式，两者不能同时存在：

- `tasks`：任务列表模式，通常用于单账号任务
- `sessions`：编排模式，主要用于多个账号按顺序轮跑

#### 任务列表模式

```yaml
version: 2
manage_environment_lifecycle: true  # 可选，默认 true
manage_game_lifecycle: true         # 可选，默认 true
account_name: main                  # 可选，注入给 StartUp

tasks:
  - type: Fight
    # ...
```

#### 编排模式

所有 Session 共享 Profile 中的 `client_type`，只有 `account_name` 不同，按顺序串行执行。

`manage_environment_lifecycle` 全局只生效一次（环境只启停一次）。
`manage_game_lifecycle` 每个 Session 执行一次：

- 为 `true` 时，运行时在各 Session 之间自动执行 CloseDown → StartUp
- 为 `false` 时，仅按书写顺序执行，如需切账号需手写 StartUp / CloseDown

在 `sessions` 模式下：

- 顶层不能出现 `account_name`
- 每个 Session 只描述该账号自己的 `tasks`
- Session 按顺序执行，当前 Session 未完成时不进入下一个

编排模式下的失败处理规则：

- 用户主动中断时，立即停止整个运行，不进入后续 Session
- 当前 Session 报错或掉线时，将该 Session 标记为失败，继续执行后续 Session
- 启用自动游戏生命周期时，当前 Session 报错或掉线后仍会先执行 CloseDown，再进入下一个 Session
- 最终结果需反映所有失败的 Session，不能因后续 Session 成功而吞掉前面的错误

```yaml
version: 2
manage_environment_lifecycle: true  # 可选，默认 true，全局一次

sessions:
  - account_name: main
    tasks:
      - type: Fight
        params:
          stage: "1-7"

  - account_name: alt
    tasks:
      - type: Fight
        params:
          stage: CE-6
```

多 Session 模式下重复配置较多，可以利用 YAML 锚点进行复用；后续也可能引入 template 机制。

### Task 定义

每个 Task 包含：

- `type`（必填）：MaaCore 任务类型
- `name`（可选）：控制最终显示在 Summary 里面的名字
- `if`（可选）：控制该任务是否执行的条件；缺省表示始终执行
- `params`（可选）：任务执行时始终传给 MaaCore 的基础参数
- `override_strategy`（可选）：多个 override 同时命中时的处理策略，`first` 或 `merge`，默认 `first`
- `overrides`（可选）：条件性参数覆盖列表，叠加在 `params` 之上

除非显式说明，V2 中的 `type` 都直接对应 MaaCore task 类型。V2 不额外引入新的"生命周期 task 类型"；自动启停由 `manage_*_lifecycle` 控制。

```yaml
tasks:
  - type: Fight
    name: Fight Daily
    override_strategy: first
    if:
      weekdays: [Mon, Wed, Fri]
    params:
      stage: "1-7"
      report_to_penguin: true
    overrides:
      - if: OnSideStory
        params:
          stage: ""
      - if:
          weekdays: [Tue, Thu, Sat]
        params:
          stage: CE-6
```

`overrides` 的应用策略由 `override_strategy` 决定：

- `first`（默认）：按顺序找到第一个命中的 override，应用后停止
- `merge`：按顺序应用所有命中的 override，后者覆盖前者

无匹配时使用原始 `params`，任务照常执行（注意，V1 版本则会静默跳过）。

---

## 条件系统

条件出现在两处：

- `task.if`——决定任务是否执行
- `override.if`——决定该 override 是否应用

### 字符串形式（无参数条件）

```yaml
if: Always       # 始终激活（缺省 if 时的默认行为）
if: OnSideStory  # SideStory 期间激活（使用 Profile 中的 client_type）
```

### `Weekday`（星期）

```yaml
if:
  weekdays: [Mon, Wed, Fri]
  timezone: Official   # 可选：Local（默认）/ Official / YoStarEN / ... / UTC 偏移整数
```

`timezone` 为客户端名称时，以服务器时间 04:00 作为换天边界。

### `DayMod`（天数取模）

```yaml
if:
  divisor: 2
  remainder: 0   # 可选，默认 0
  timezone: 8    # 可选，UTC 偏移整数
```

### `Time`（每日时间段，循环）

```yaml
if:
  time_range:
    from: "16:00:00"
    until: "23:59:59"
  timezone: Official   # 可选
```

仅设 `from`：从该时刻激活直到当天结束。仅设 `until`：从当天开始激活到该时刻。支持跨午夜的时间段（如 23:00–01:00）。

### `DateTime`（日期范围，一次性）

```yaml
if:
  date_range:
    from: "2024-08-01T00:00:00"
    until: "2024-08-21T04:00:00"
  timezone: 8   # 可选，UTC 偏移整数
```

### 组合条件

```yaml
# AND——所有子条件满足
if:
  all:
    - weekdays: [Mon, Wed, Fri]
    - time_range:
        from: "16:00:00"

# OR——任一子条件满足
if:
  any:
    - weekdays: [Sat]
    - weekdays: [Sun]

# NOT——取反
if:
  not:
    weekdays: [Sun]
```

---

## 输入配置

任务参数中的字段可以定义为"输入值"，在实际运行前由解析器处理。有两种运行模式：

- **交互模式**（默认）：在终端向用户逐一提问，等待手动输入。
- **非交互模式**（`--batch`）：跳过所有交互提问，直接使用默认值；配合 `-D<id>=<value>` 可按字段 `id` 注入预设答案覆盖默认值。

输入值通过 `default` 字段（或 `alternatives` + `default_index`）与普通字面量区分。
`id` 字段是非交互模式下注入答案的稳定标识符；`description` 字段是交互时显示给用户的说明文字。

### 自由输入

适用于字符串、整数、浮点数类型，由 `default` 值的类型自动推断：

```yaml
params:
  stage:
    id: stage
    default: "1-7"
    description: "要刷的关卡"
```

**交互模式**提示格式（输入后按回车；直接回车使用默认值；输入无法解析时重新提问）：

```plain
Please input 要刷的关卡 [default: 1-7]: 
```

**非交互模式**：

```shell
maa run daily --batch -Dstage=CE-5
# stage → "CE-5"；未提供 -D 时使用默认值 "1-7"
```

### 确认输入

适用于布尔类型，`default` 为 `true` 或 `false`：

```yaml
params:
  enabled:
    id: enabled-fight
    default: true
    description: "开启此功能"
```

**交互模式**提示格式（`default: true` 显示 `[Y/n]`，`default: false` 显示 `[y/N]`）：

```plain
Whether to 开启此功能 [Y/n]: 
```

接受 `y`/`yes`/`true`（大小写不敏感）→ `true`；`n`/`no`/`false` → `false`。

**非交互模式**：

```shell
maa run daily --batch -Denabled-fight=false
# enabled → false
```

### 列表选择

从预设备选项中选择，`default_index` 为 1-based 的默认选项序号：

```yaml
params:
  stage:
    id: stage
    alternatives:
      - value: CE-5
        desc: "龙门币关卡 5"
      - value: CE-6
        desc: "龙门币关卡 6"
    default_index: 2
    allow_custom: true   # 允许输入不在列表中的自定义值
    description: "要刷的关卡"
```

`alternatives` 也可以是纯值列表（不带 `desc`）：

```yaml
alternatives: [CE-5, CE-6, 1-7]
```

**交互模式**提示格式：

```plain
1. CE-5 (龙门币 5)
2. CE-6 (龙门币 6) [default]
Please select 要刷的关卡 or input a custom value (empty for default): 
```

输入 1-based 序号选择对应项；启用 `allow_custom: true` 后可直接输入自定义值；直接回车使用默认项。

**非交互模式**（传入序号或自定义值）：

```shell
maa run daily --batch -Dstage=1    # 按序号：stage → "CE-5"
maa run daily --batch -Dstage=CE-4 # 自定义值（需启用 allow_custom）：stage → "CE-4"
```

### 条件输入

`conditions`（别名 `deps`）字段让某个参数仅在同对象内的依赖字段满足期望值时才出现：

```yaml
params:
  report_to_penguin:
    id: report_to_penguin
    default: false
    description: "上报企鹅物流"
  penguin_id:
    conditions:
      report_to_penguin: true   # 仅在 report_to_penguin == true 时出现
    id: penguin_id
    default: ""
    description: "企鹅物流 ID"
```

- 条件不满足时，该字段从结果中完全省略；
- 支持链式依赖，运行时按依赖顺序解析；
- 循环依赖会在运行时报错。

**非交互模式**示例：

```shell
maa run daily --batch -Dreport_to_penguin=true -Dpenguin_id=12345678
```

---

## 完整 Task 示例

```yaml
version: 2
manage_game_lifecycle: true

tasks:
  # Fight：始终执行，但根据条件覆盖关卡
  - type: Fight
    params:
      stage: "1-7"
      report_to_penguin: true
      penguin_id: "00000000"
    overrides:
      - if: OnSideStory
        params:
          stage: ""
      - if:
          weekdays: [Tue, Thu, Sat]
        params:
          stage: CE-6

  # Mall：仅在 16:00 后执行
  - type: Mall
    if:
      time_range:
        from: "16:00:00"
    params:
      shopping: true
      credit_fight: true
      buy_first: [招聘许可, 龙门币]
      blacklist: [碳, 家具, 加急许可]

  # Recruit：始终执行
  - type: Recruit
    params:
      refresh: true
      select: [4, 5]
      confirm: [3, 4, 5]
```

---

## V1 → V2 迁移

### Profile 迁移

#### V1

```toml
[connection]
type = "ADB"
adb_path = "adb"
device = "emulator-5554"
config = "CompatMac"

[resource]
global_resource = "YoStarEN"
user_resource = true

[static_options]
cpu_ocr = true

[instance_options]
touch_mode = "MaaTouch"
deployment_with_pause = false
adb_lite_enabled = false
kill_adb_on_exit = false
```

#### V2

```toml
version = 2
client_type = "YoStarEN"       # resource.global_resource 不再需要手动设，由 client_type 自动推导

[connection]
type = "General"               # "ADB" → "General"
address = "emulator-5554"      # "device" → "address"
adb_path = "adb"
touch_mode = "MaaTouch"        # 原 instance_options.touch_mode
adb_lite = false               # 原 instance_options.adb_lite_enabled
kill_adb_on_exit = false
config = "CompatMac"

[behavior]
deployment_with_pause = false  # 原 instance_options.deployment_with_pause

[advanced]
inference_engine = "cpu"       # 原 static_options.cpu_ocr = true → "cpu"
user_resource = true           # 原 resource.user_resource
```

主要变化：

- 添加 `version = 2`
- `connection.type`：`ADB` → `General`
- `connection.device` → `connection.address`
- `resource.global_resource` / `platform_diff_resource` → 由 `client_type` 和 `connection.type` 自动推导，不再需要手动设
- `static_options` / `instance_options` → 拆分到 `[connection]`、`[behavior]`、`[advanced]`

### Task 迁移

#### V1

```yaml
tasks:
  - type: StartUp
    params:
      start_game_enabled: true
      client_type: Official

  - type: Fight
    params:
      stage: "1-7"
    variants:
      - condition:
          type: Weekday
          weekdays: [Tue, Thu, Sat]
        params:
          stage: CE-6
      - condition:
          type: Always
        params: {}

  - type: CloseDown
```

#### V2

```yaml
version: 2
manage_game_lifecycle: true    # 替代手写 StartUp / CloseDown task

tasks:
  - type: Fight
    params:
      stage: "1-7"             # base params，始终使用
    overrides:                 # variants → overrides
      - if:
          weekdays: [Tue, Thu, Sat]   # condition → if，无需 type 标签
        params:
          stage: CE-6          # 不再需要 Always variant 保底
```

主要变化：

- `StartUp` / `CloseDown` task → `manage_game_lifecycle: true`（默认值），自动管理游戏启停
- `client_type` 不再在 task params 中设置，由 Profile 注入
- 用户仍可手写 StartUp / CloseDown（设 `manage_game_lifecycle: false`），但 `client_type` 只注入不提取，且运行时完全按书写顺序执行
- `variants` → `overrides`，仅覆盖参数，不影响任务是否执行
- `condition` → `if`，通过唯一字段推断类型，无需 `type` 标签
- 不再需要 `Always` variant 作为保底

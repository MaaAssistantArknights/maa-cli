## CLI help messages

-maa = MAA（明日方舟小助手）

## Headings (case is important)
SUBCOMMAND = 子命令
Subcommands = 子命令
Arguments = 参数
Options = 选项
Global-Options = 全局选项
Usage = 用法

## Global options

about = {-maa}命令行工具

batch-help = 启动非交互模式
batch-long-help = 启动非交互模式，所有需要输入的值都会设定为默认值，如果某一项没有默认值，则报错。

verbose-help = 提高日志输出等级
quiet-help = 降低日志输出等级

log-file-help = 重定向日志输出到文件
log-file-long-help = 重定向日志输出到文件，如果不指定路径，根据当前时间写入日志目录。
   每次运行的日志会写入到一个新的文件中，路经为 YYYY/MM/DD/HH:MM:SS.log。

help-help = 显示当前命令的帮助信息 (--help 显示更详细的信息)

version-help = 显示版本信息

## Subcommands

## Some common used options
-channel-help = 更新通道，可选值为: stable（默认）、beta、alpha
-api-url-help = 获取最新版本信息的 API 地址
-api-url-long-help = {-api-url-help}。{-opt-set-in-config}

## Phrases
-opt-set-in-config = 该选项也可以在配置文件中设置

## maa install
install-about = 安装 MaaCore 共享库及其资源
install-force-help = 强制安装 MaaCore 库及其资源
install-force-long-help = 强制安装最新版 MaaCore 库及其资源，如果已经安装，会覆盖原有文件。

## maa update
update-about = 更新 MaaCore 及其资源

## Common options for maa install and maa update
core-channel-help = {-channel-help}
core-channel-long-help = {core-channel-help}，alpha 仅 Windows 可用。
    {-opt-set-in-config}。
core-no-resource-help = 不安装资源
core-no-resource-long-help = 不安装 MaaCore 的资源。{-opt-set-in-config}。
core-test-time-help = 对镜像测速的时长 (单位: 秒)
core-test-time-long-help = 对镜像测速的时长 (单位: 秒)，设置为 0 则跳过测速，直接从 GitHub 下载。
    {-opt-set-in-config}。
core-api-url-help = {-api-url-help}
core-api-url-long-help = {-api-url-long-help}。
    默认为 `https://ota.maa.plus/MaaAssistantArknights/api/version/`。

self-update-about = 更新 maa-cli 自身
self-channel-help = {-channel-help}
self-channel-long-help = {self-channel-help}，alpha 为夜间构建版本。
   {-opt-set-in-config}。
self-api-url-help= {-api-url-help}
self-api-url-long-help = {-api-url-long-help}。
    默认为 `https://github.com/MaaAssistantArknights/maa-cli/raw/version/`。
self-download-url-help = 预编译 CLI 的下载地址
self-download-url-long-help = {self-download-url-help}。
    默认为 `https://github.com/MaaAssistantArknights/maa-cli/releases/download/`。
    {-opt-set-in-config}。

hot-update-about = 更新和 MaaCore 版本不依赖的资源

dir-about = 获得 MAA 相关的目录
dir-target-help = 目录类型，可选值为：config、data、library、cache、resource、hot-update、log
dir-target-long-help = 目录类型：
    config：用户配置目录。
    data：数据目录，MaaCore 及其资源的默认安装在此目录下。
    library：MaaCore 库目录，包括 MaaCore 的库目录和用户配置目录。
    resource：MaaCore 的资源目录。
    hot-update：热更新资源目录。
    cache：缓存目录，包括下载的安装包和自动战斗作业文件。
    log：日志目录，包括 MaaCore 和 CLI 的日志，以及部份用于调试的文件。

version-about = 获得 maa-cli 以及 MaaCore 的版本信息
version-component-help = 需要获得的版本信息组件。可选值为：cli、core、all。默认为 all。

run-about = 运行自定义任务
run-task-help = 自定义任务的名称
run-task-long-help = {run-task-help}。使用 `maa list` 命令查看所有可用的任务。
run-addr-help = 连接游戏的地址
run-addr-long-help = {run-addr-help}。

    使用 ADB 连接游戏时，这个地址可以是序列号也可以是IP+端口的地址，默认为 `emulator-5554`。
    使用 PlayTools 连接游戏时，在 Playtools 中设置的地址，默认为 `localhost:1717`。

    {-opt-set-in-config}。
run-user-resource-help = 加载用户配置目录下的资源
run-user-resource-long-help = {run-user-resource-help}。

    默认情况下 CLI 会自动寻找并加载 MaaCore 一同安装的资源以及热更新资源。
    如果你希望更改这些资源文件中，可以在用户配置目录下创建资源目录并创建你的资源文件。
    然后使用这个选项来加载你的资源文件。在开启这个选项后，CLI 会最后加载你的资源文件，
    这样你的资源文件中的内容会覆盖默认的资源文件中的内容。

    {-opt-set-in-config}。
run-dry-run-help = 仅解析配置文件，不实际运行任务
run-dry-run-long-help = {run-dry-run-help}。

    在测试任务时，CLI 会解析你的配置文件，然后尝试向 MaaCore 发送指令，但不会启动 MaaCore。
    你可能需要同时调整日志输出等级来查看相关信息。建议组合使用 `--dry-run -vv` 来进行测试。
run-no-summary-help = 任务结束后不显示任务总结
run-no-summary-long-help = {run-no-summary-help}。

    在任务结束后，CLI 会显示任务总结，包括每个任务的运行时间，以及部份支持任务的结果。
    如果你不希望看到这些信息，可以使用这个选项来关闭任务总结。

startup-about = 启动游戏到主界面
startup-client-help = 游戏客户端的类型，留空则不启动客户端，直接连接游戏。
    可选值为：Official、Bilibili、YoStartEN、YoStartJP、YoStartKR、Txwy。
startup-client-long-help = 游戏客户端的类型，留空则不启动客户端，直接连接游戏。

    - Official：官服；
    - Bilibili：B服；
    - YoStartEN：英文服；
    - YoStartJP：日文服；
    - YoStartKR：韩文服；
    - Txwy：台服。

closedown-about = 关闭游戏客户端

fight-about = 自动刷图
fight-stage-help = 关卡名称，如 1-7
fight-stage-long-help = {fight-stage-help}，支持所有主线关卡，资源关卡，剿灭和部份活动关卡。
    留空则自动选择当前或上一次刷的关卡。使用 `maa activity` 查看支持的关卡。

copilot-about = 自动战斗
copilot-uri-help = 自动战斗作业文件的本地路径或者 URI，如 `1234.json` 或 `maa://1234`。

roguelike-about = 自动集成战略（肉鸽）
roguelike-theme-help = 集成战略的主题，可选值为：Phantom、Mizuki、Sami
roguelike-theme-long-help = {roguelike-theme-help}。

    - Phantom：傀影与猩红孤钻；
    - Mizuki：水月与深蓝之树；
    - Sami：探索者的银凇止境。

convert-about = 转换配置文件格式，支持 JSON、YAML、TOML
convert-input-help = 输入文件的路径
convert-output-help = 输出文件的路径，留空则输出到标准输出流
convert-format-help = 输出的格式，可选值为：json、yaml、toml，可以缩写为 j、y、t
convert-format-long-help = {convert-format-help}。

    当指定了输出文件的路径时，会根据文件后缀名自动推断输出的格式。
    当没有指定输出文件的路径时，必须指定输出的格式。
    当两者都指定时，会根据指定的格式改变输出文件的后缀名。

activity-about = 查看当前游戏的活动信息
activity-client-help = 游戏客户端的类型，可选值为：Official（默认）、YoStartEN、YoStartJP、YoStartKR、Txwy

list-about = 列出所有可用的自定义任务
complete-about = 生成自动补全脚本
complete-shell-help = 生成的自动补全脚本的 shell 类型，可选值为：bash、zsh、fish、powershell


## Installer output messages

fetching = 正在获取 { $name } 最新版本信息，更新通道：{ $channel }
failed-fetch-version-json = 从 { $url } 获取版本信息文件失败
failed-parse-version-json = 解析版本信息文件失败

unsupported-architecture = 不支持的架构：{ $arch }
unsupported-platform = 不支持的平台：{ $arch } { $os }

asset-not-found = 未找到安装包: { $name }

update-to-date = 当前已经是最新版本：{ $name } { $version }
failed-parse-version = 解析的版本号失败
found-newer-version = 发现新版本：{ $name } { $old } -> { $new }
package-cache-hit = 在缓存中找到安装包：{ $file }，跳过下载
downloading = 正在下载安装包：{ $file }
installing = 正在安装：{ $name } { $version }

skip-speed-test = 跳过镜像测速，使用默认地址 { $link }
testing-download-speed = 正在测试下载速度
testing-mirror = 正在测试镜像：{ $link }
found-faster-mirror = 发现更快的镜像：{ $link }
download-from-fastest-mirror = 从最快的镜像下载：{ $link }

downloaded-verifying = 下载完成，正在验证
downloaded = 下载完成
failed-download = 下载 { $file } 失败
verified = 验证通过
failed-verify = 验证失败

unsupported-archive = 不支持的压缩格式：{ $file }
unknown-archive = 未知的压缩格式：{ $file }
extracting = 正在解压缩
extract = 解压缩 { $src } 到 { $dest }
skip-extract = 跳过文件 { $src }
extracted = 解压缩完成
failed-extract = 解压缩 { $file } 失败
failed-remove = 删除 { $file } 失败
failed-symlink = 创建软链接 { $file } 失败
failed-create = 创建 { $file } 失败
failed-write = 写入 { $file } 失败
failed-set-permission = 设置 { $file } 权限失败
failed-read-entry = 读取压缩包中的文件失败

create-dir = 创建目录 { $dir }
remove-dir = 删除目录 { $dir }

core-already-installed = MaaCore 已经安装，请使用 `maa update` 命令更新 MaaCore，或者使用 `maa install --force` 命令强制安装。
library-installed-by-other = 已在 { $path } 找到到安装的 MaaCore 库，但是不是由 maa-cli 安装的，maa-cli 无法管理这个库。
resource-installed-by-other = 已在 { $path } 找到安装的资源，但是不是由 maa-cli 安装的，maa-cli 无法管理这个资源。
deprecated-disable-library-option = 不安装 MaaCore 库的选项已经废弃，因为不建议分开安装 MaaCore 和资源。如果你有必须要这么做的理由，请前往 GitHub 提交 issue。
deprecated-disable-resource-option = 不安装 MaaCore 资源的选项已经废弃，因为不建议分开安装 MaaCore 和资源。如果你想获得最新的资源，可以使用 `maa hot-update` 命令。
no-component-to-install = 没有需要安装的 MaaCore 组件，跳过安装，仅安装部份 MaaCore 组件的选项已经废弃。如果你有必须要这么做的理由，请前往 GitHub 提交 issue。

updating-resource-repository = 正在更新资源仓库
cloning-resource-repository = 正在克隆资源仓库
failed-open-resource-repository = 打开资源仓库失败
failed-clone-resource-repository = 克隆资源仓库失败
failed-pull-resource-repository = 更新资源仓库失败
failed-find-remote = 未找到远程仓库：'{ $name }'
failed-find-reference = 未找到引用：{ $name }
failed-reference-to-annotated-commit = 从引用 { $name } 创建提交失败
failed-merge-analysis = 分析合并失败
failed-create-reference = 创建引用 { $name } 失败
failed-set-head = 设置 HEAD 失败
fast-forward-merge = 快进合并
failed-merge = 合并失败
failed-checkout = 切换到 { $name } 失败
repo-up-to-date = 仓库已经是最新版本

## Start game messages
game-is-running = 游戏正在运行
start-game = 启动游戏: { $name }
stop-game = 停止游戏: { $name }
game-ready = 游戏启动完成
waiting-for-game = 等待游戏启动
failed-connect-game = 连接游戏失败

## Load maa-core messages
maa-core-already-loaded = MaaCore 已经加载
load-maa-core = 从 { $path } 加载 MaaCore
maa-core-not-found = 未找到安装的 MaaCore
use-system-maa-core = 未找到 MaaCore，尝试从系统路径加载

## Load config messages
failed-load-config-skip = 加载配置文件 { $path } 失败，跳过，错误信息：{ $error }
no-successful-config-found-use-default = 没有找到可用的配置文件，使用默认配置

detected-client-type = 检测到游戏客户端类型：{ $client }
detected-connection-type = 检测到连接类型：{ $connection }

set-user-directory = 设置用户目录：{ $path }
failed-set-user-directory = 设置用户目录为 { $path } 失败

## Load resource messages
found-resource-directory = 找到资源目录：{ $path }
resource-directory-not-found = 未找到资源目录
found-hot-update-resource = 找到热更新资源目录：{ $path }
hot-update-resource-not-exist = 热更新资源不存在：{ $path }
-gloablize-resource = 非简中服资源
use-globalize-resource = 加载 { $path } 中的 { -gloablize-resource }
globalize-resource-twice-set = { -gloablize-resource } { $loaded } 已经加载，跳过加载 { $path }
globalize-resource-not-found = 未找到 { -gloablize-resource } { $path }，跳过加载
-platefrom-diff-resource = 平台差异资源
use-platform-diff-resource = 加载 { $path } 中的 { -platefrom-diff-resource }
platform-diff-resource-twice-set = { -platefrom-diff-resource } { $loaded } 已经加载，跳过加载 { $path }
platform-diff-resource-not-found = 未找到 { -platefrom-diff-resource } { $path }，跳过加载
load-resource-from = 从 { $path } 加载资源
resource-not-exist = 资源目录 { $path } 不存在，跳过加载

## Set static options messages
both-cpu-and-gpu-ocr-enabled = 同时启用 CPU 和 GPU OCR，CPU OCR 会被禁用
use-cpu-ocr = 使用 CPU OCR
failed-use-cpu-ocr = 使用 CPU OCR 失败
use-gpu-ocr = 使用 GPU { $id } OCR
failed-use-gpu-ocr = 使用 GPU { $id } OCR 失败

## Set instance options messages
automatic-macplaytools = 自动选择触摸模式为 MacPlayTools
force-macplaytools = 使用 PlayTools 连接游戏，强制设置触摸方式为 MacPlayTools
force-disable-adb-lite = 使用 PlayTools 连接游戏，强制禁用 ADB Lite
set-touch-mode = 设置触摸模式为 { $mode }
failed-set-touch-mode = 设置触摸模式为 { $mode } 失败
deploy-with-pause = { $enabled ->
    [true] 设置部署时暂停游戏
    *[false] 设置部署时不暂停游戏
}
failed-deploy-with-pause = { $enabled ->
    [true] 设置部署时暂停游戏失败
    *[false] 设置部署时不暂停游戏失败
}
adb-lite = { $enabled ->
    [true] 启用 ADB Lite
    *[false] 禁用 ADB Lite
}
failed-adb-lite = { $enabled ->
    [true] 启用 ADB Lite 失败
    *[false] 禁用 ADB Lite 失败
}
kill-adb-on-exit = { $enabled ->
    [true] 设置退出时关闭 ADB
    *[false] 设置退出时不关闭 ADB
}
failed-kill-adb-on-exit = { $enabled ->
    [true] 设置退出时关闭 ADB 失败
    *[false] 设置退出时不关闭 ADB 失败
}

## Connection messages
connection-args-adb = 使用配置 { $config } 连接 { $device } (ADB 为 { $adb })
connection-args-playtools = 使用配置 { $config } 连接 { $address }

## Task messages
unknown-task-type = 未知的任务类型：{ $task_type }，请检查你的任务类型是否正确，如果这是一个新的任务类型，而非拼写错误，请前往 GitHub 提交 issue。
task-type-startup = 启动游戏
task-type-closedown = 关闭游戏
task-type-fight = 刷理智
task-type-recruit = 公开招募
task-type-infrast = 基建换班
task-type-mall = 领取信用及商店购物
task-type-award = 领取日常奖励
task-type-roguelike = 自动集成战略
task-type-copilot = 自动抄作业
task-type-ssscopilot = 自动保全派驻
task-type-reclamationalgorithm = 自动生息演算
task-type-depot = 仓库识别
task-type-operbox = 干员 Box 识别
task-type-custom = 自定义任务
task-type-singlestep = 单步任务
task-type-videorecognition = 视频识别

failed-find-task-file = 未找到任务文件：{ $file }
task-directory-not-exist = 任务目录不存在：{ $path }
append-task-with-param = 添加任务 [{ $task }]，参数为：{ $params }
append-task-no-param = 添加任务 [{ $task }]

## Copilot messages
failed-find-stage-file = 未找到关卡 { $stage } 的信息文件，你的资源可能过期了，请更新资源。
failed-parse-stage-info = 解析关卡信息失败, { $info }

copilot-cache-hit = 在缓存中找到自动战斗作业文件：{ $file }，跳过下载
failed-download-copilot = 从 { $url } 下载自动战斗作业失败
failed-parse-copilot = 解析自动战斗作业失败
copilot-downloaded = 从 { $url } 下载自动战斗作业完成
failed-get-copilot-content = 获取自动战斗作业内容失败
failed-parse-copilot-content = 解析自动战斗作业内容失败

failed-get-stage-name = 获取关卡名称失败
failed-get-group-name = 获取干员组名称失败
failed-get-operator-name = 获取干员名称失败

copilot-stage = 自动战斗关卡：
copilot-operators = 干员列表：

## Callback messages

## Top level messages
init-failed = MaaCore 初始化失败
all-tasks-completed = 所有任务完成
failed-process-message = 处理回调消息失败，消息代码：{ $code }，消息内容：{ $message }

## connection info messages
got-resolution = 获取分辨率成功：{ $width }x{ $height }
failed-get-resolution = 获取分辨率失败
unsupported-resolution = 不支持的分辨率（{ $width }x{ $height }）: { $why }
low-screen-resolution = 分辨率过低（{ $width }x{ $height }）
not-16-9 = 非 16:9 分辨率（{ $width }x{ $height }）

connected = 已连接到 { $address }
disconnected = 连接断开
reconnecting = 正在尝试第 { $times } 次重连
reconnected = 重连成功

failed-screencap = 截图失败
fastest-way-screencap = 最快的截图方式为 { $method } 消耗时间：{ $cost }ms
screencap-cost = 最近十次截图消耗时间为 { $min }ms ~ { $max }ms，平均 { $avg }ms

touch-mode-not-available = 触摸模式不可用
unknown-connection-info = 未知的连接信息：{ $message }

## Task chain messages

taskchain-start = 任务链 { $name } 开始
taskchain-completed = 任务链 { $name } 完成
taskchain-stopped = 任务链 { $name } 被中断
taskchain-error = 任务链 { $name } 出错

## Subtask error
failed-start-game = 启动游戏失败
failed-auto-recruit = 自动公招失败，{ $why }
failed-recognize-drops = 识别掉落物品失败
failed-report-penguinstats = 上报企鹅物流失败，{ $why }
failed-report-penguinstats-unknown-drops = 上报企鹅物流失败，未知的掉落物品
failed-report-penguinstats-unknown-drop-type = 上报企鹅物流失败，未知的掉落类型
failed-report-yituliu = 上报一图流失败, { $why }
failed-report-yituliu-unknown-drops = 上报一图流失败，未知的掉落物品
failed-report-yituliu-unknown-drop-type = 上报一图流失败，未知的掉落类型
invalid-stage-for-recognition = 无法识别掉落，{ $why }
unknown-subtask-error = 未知的子任务错误：{ $message }

## Substart start

game-offline = 游戏掉线

## mission
mission-start = 开始作战
mission-start-times = 开始第 { $times } 次作战
medicine-used = 第 { $times } 次使用理智药
stone-used = 第 { $times } 次碎石换理智

prts-error = 代理作战失误，放弃本次作战

## Recruit
recruit-refresh = 刷新公招标签
recruit-confirm = 确认招募

## Infrastucture
infrast-dorm-double-confirm = 干员冲突

## RogueLike

roguelike-start = 开始第 { $times } 次探索
roguelike-abandon = 放弃本次探索
roguelike-complete = 探索完成，通关了！

invest = 第 { $times } 次投资
invest-full = 投资达到上限，无法继续投资
special-item-bought = 购买了特殊商品！

mission-complete = 作战完成
mission-failed = 作战失败
trader-enter = 进入商店
safe-house-enter = 进入安全屋
normal-dps-enter = 进入普通作战
emergency-dps-enter = 进入突袭作战
dreadful-foe-enter = 进入险路恶敌

unknown-subtask-start = 未知的子任务开始：{ $message }

## Subtask extra info

depot-recognition = 仓库识别结果：{ $result }
operator-recognition = 干员识别结果：{ $result }

drops = 本次作战掉落物品：{ $drops }
sanity-before-stage = 当前理智：{ $sanity } / { $max }

facility-enter = 进入设施：{ $facility }#{ $index }
product-of-facility = 设施 { $facility }#{ $index } 产物：{ $product }
product-incorrect = 设施 { $facility }#{ $index } 产物不正确 { $product }
product-changed = 设施 { $facility }#{ $index } 产物变更为：{ $product }

not-enough-staff = 没有足够的干员进驻设施 { $facility }#{ $index }

custom-infrast-operators = 设施 { $facility }#{ $index } 进驻干员：{ $operators }
custom-infrast-candidates = 设施 { $facility }#{ $index } 进驻备选干员：{ $candidates  }
custom-infrast-both = 设施 { $facility }#{ $index } 进驻干员：{ $operators }，备选干员：{ $candidates }
## TODO: the following messages are not handled yet
## CustomInfrastRoomGroupsMatch
## CustomInfrastRoomGroupsMatchFailed

## Facility
Control = 控制中枢
Mfg = 制造站
Trade = 贸易站
Power = 发电站
Office = 办公室
Reception = 会客室
Dorm = 宿舍
Processing = 加工站
Training = 训练室
UnknownFacility = 未知设施

## Product
LMD = 龙门币
PureGold = 赤金
Orundum = 合成玉
OriginiumShard = 源石碎片
Dualchip = 双芯片
BattleRecord = 作战记录
UnknownProduct = 未知产物

## Recruit
recruit-tags = 发现 { $star } 星标签: { $tags }
recruit-special-tag = 发现特殊标签：{ $tag }
recruit-robot-tag = 发现小车标签：{ $tag }
recruit-tags-selected = 已选择标签：{ $tags }
recruit-no-permit = 招聘许可不足，无法招募

## RogueLike
roguelike-stage-enter = 进入关卡：{ $name }
roguelike-stage-info-error = 关卡识别错误
roguelike-event = 发现事件：{ $name }

roguelike-pass = 集成战略通关
roguelike-fail = 集成战略失败
roguelike-settlement = 难度 { $difficulty } { $pass }：
    通过 { $explore } 层，前进 { $steps } 步，
    普通战斗 { $combat } 次，精英战斗 { $emergency } 次，领袖战斗 { $boss } 次，
    招募 { $recruit } 次，收集 { $object } 个藏品，
    得分 { $score }，获得 { $exp } 经验值，获得 { $skill } 技能点。


## Copilot
Copilot = 自动战斗
SSSCopilot = 自动保全派驻

battle-formation = 编队：{ $formation }
battle-formation-selected = 选中干员：{ $selected }
current-copilot-action = 当前自动战斗动作：{ $action } { $target } { $doc }
unsupported-level = 不支持的关卡，请检查关卡名！

Deploy = 部署
UseSkill = 使用技能
Retreat = 撤退
SwitchSpeed = 二倍速
BulletTime = 子弹时间
SkillUsage = 技能使用
Output = 输出
SkillDaemon = 摆完挂机
MoveCamera = 移动镜头
DrawCard = 抽卡
CheckIfStartOver = 检查是否重开

## SSS
sss-stage-enter = 进入关卡：{ $name }
sss-settlement = 保全派驻结算：{ $why }
sss-game-pass = 保全派驻通关

unknown-subtask-extra-info = 未知的子任务信息：{ $message }

## Task Summary

task-summary = 任务总结

task-state-unstarted = 未开始
task-state-unfinished = 已开始，未完成
task-state-completed = 已完成
task-state-stopped = 已中断
task-state-error = 出错

summary-infrast-operator = 进驻干员：{ $operators }
summary-infrast-candidate = 进驻备选干员：{ $candidates }
summary-infrast-both = 进驻干员：{ $operators }，备选干员：{ $candidates }

summary-fight-stage = 刷 { $stage }
summary-fight-times = { $times } 次
summary-fight-medicine = ，使用 { $medicine } 瓶理智药
summary-fight-stone = ，碎石 { $stone } 个
summary-fight-drop = ，掉落记录如下：
summary-fight-total-drop = 总计掉落：

recruit-refreshed = 已刷新
recruit-recruited = 已招募
recruit-refreshed-times = 刷新 { $times } 次
recruit-recruited-times = 招募 { $times } 次
recruit-tag-records = 标签记录如下：

roguelike-explore-times = 探索 { $times } 次
roguelike-invest-times = 投资 { $times } 源石锭

## Common error messages

## Network error messages
failed-create-reqwest-client = 创建 reqwest 客户端失败
failed-send-request = 向 { $url } 发送请求失败
failed-response-status = 请求失败，状态码为：{ $status }

## File error messages
failed-open-file = 打开文件 { $file } 失败
failed-read-file = 读取文件 { $file } 失败
failed-write-file = 写入文件 { $file } 失败

## Json error messages
failed-deserialize-json = 反序列化 JSON 失败
failed-serialize-json = 序列化 JSON 失败
unknown-value = 未知的值：{ $value }
value-type-mismatch = { $value } 的值必须为 { $expected }

## convert error messages
invalid-utf8-path = 路径中包含无效的 UTF-8 字符

# Async error messages
failed-create-tokio-runtime = 创建 tokio 运行时失败
failed-register-signal-handler = 注册信号处理器失败
interrupted = 用户中断

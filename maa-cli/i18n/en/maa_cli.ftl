-maa = MAA (MaaAssistantArknights)

## Headings (case is important)
SUBCOMMAND = Subcommand
Subcommands = Subcommands
Arguments = Arguments
Options = Options
Global-Options = Global Options
Usage = Usage

## Global options

about = { -maa } Command-line tool

batch-help = Start non-interactive mode
batch-long-help = Start non-interactive mode, all values requiring input will be set to default values. If a certain item has no default value, an error will be reported.

verbose-help = Increase log output level
quiet-help = Decrease log output level

log-file-help = Redirect log output to a file
log-file-long-help = Redirect log output to a file. If no path is specified, it will be written to the log directory based on the current time. Each run's log will be written to a new file, with the path being YYYY/MM/DD/HH:MM:SS.log.

help-help = Display help information for the current command (--help displays more detailed information)

version-help = Display version information

## Subcommands

## Some common used options
-channel-help = Update channel, optional values are: stable (default), beta, alpha
-api-url-help = API address for obtaining the latest version information
-api-url-long-help = { -api-url-help }. { -opt-set-in-config }

## Phrases
-opt-set-in-config = This option can also be set in the configuration file

## maa install
install-about = Install MaaCore shared library and its resources
install-force-help = Force installation of MaaCore library and its resources
install-force-long-help = Force installation of the latest version of MaaCore library and its resources. If already installed, it will overwrite existing files.

## maa update
update-about = Update MaaCore and its resources

## Common options for maa install and maa update
core-channel-help = { -channel-help }
core-channel-long-help = { core-channel-help }, alpha only available on Windows. { -opt-set-in-config }.
core-no-resource-help = Do not install resources
core-no-resource-long-help = Do not install resources of MaaCore. { -opt-set-in-config }.
core-test-time-help = Duration of image speed test (unit: seconds)
core-test-time-long-help = Duration of image speed test (unit: seconds), set to 0 to skip speed test and download directly from GitHub. { -opt-set-in-config }.
core-api-url-help = { -api-url-help }
core-api-url-long-help = { -api-url-long-help }. Default is `https://ota.maa.plus/MaaAssistantArknights/api/version/`.

self-update-about = Update maa-cli itself
self-channel-help = { -channel-help }
self-channel-long-help = { self-channel-help }, alpha is the nightly build version. { -opt-set-in-config }.
self-api-url-help = { -api-url-help }
self-api-url-long-help = { -api-url-long-help }. Default is `https://github.com/MaaAssistantArknights/maa-cli/raw/version/`.
self-download-url-help = Download address for precompiled CLI
self-download-url-long-help = { self-download-url-help }. Default is `https://github.com/MaaAssistantArknights/maa-cli/releases/download/`. { -opt-set-in-config }.

hot-update-about = Update resources independent of MaaCore version

dir-about = Get directories related to MAA
dir-target-help = Directory type, optional values are: config, data, library, cache, resource, hot-update, log
dir-target-long-help = Directory type:
    config: User configuration directory.
    data: Data directory, default installation location for MaaCore and its resources.
    library: MaaCore library directory, including MaaCore's library directory and user configuration directory.
    resource: Resource directory of MaaCore.
    hot-update: Hot update resource directory.
    cache: Cache directory, including downloaded installation packages and auto-battle job files.
    log: Log directory, including logs for MaaCore and CLI, and some files used for debugging.

version-about = Get version information for maa-cli and MaaCore
version-component-help = Version information components to be obtained. Optional values are: cli, core, all. Default is all.

run-about = Run custom tasks
run-task-help = Name of the custom task
run-task-long-help = { run-task-help }. Use `maa list` command to view all available tasks.
run-addr-help = Address to connect to the game
run-addr-long-help = { run-addr-help }.

    When connecting to the game with ADB, this address can be a serial number or an IP + port address, default is `emulator-5554`.
    When connecting to the game with PlayTools, it is the address set in Playtools, default is `localhost:1717`. { -opt-set-in-config }.
run-user-resource-help = Load resources under the user configuration directory
run-user-resource-long-help = { run-user-resource-help }.

    By default, CLI will automatically find and load resources installed with MaaCore as well as hot-update resources.
    If you want to change these resource files, you can create a resource directory in the user configuration directory and create your resource file.
    Then use this option to load your resource file. After enabling this option, CLI will load your resource file last,
    so the contents of your resource file will override the contents of the default resource file. { -opt-set-in-config }.
run-dry-run-help = Parse the configuration file only, do not run the task actually
run-dry-run-long-help = { run-dry-run-help }.

    During testing tasks, CLI will parse your configuration file, then try to send instructions to MaaCore, but will not start MaaCore.
    You may need to adjust the log output level to view relevant information. It is recommended to use `--dry-run -vv` in combination for testing.
run-no-summary-help = Do not display task summary after the task ends
run-no-summary-long-help = { run-no-summary-help }.

    After the task ends, CLI will display a task summary, including the running time of each task, and some results of tasks that support it.
    If you do not want to see this information, you can use this option to turn off the task summary.

startup-about = Start the game to the main interface
startup-client-help = Type of game client, leave blank to not start the client and connect directly to the game.
    Optional values are: Official, Bilibili, YoStartEN, YoStartJP, YoStartKR, Txwy.
startup-client-long-help = Type of game client, leave blank to not start the client and connect directly to the game.

    - Official: Official server;
    - Bilibili: Bilibili server;
    - YoStartEN: English server;
    - YoStartJP: Japanese server;
    - YoStartKR: Korean server;
    - Txwy: Taiwanese server.

closedown-about = Close the game client

fight-about = Automatic battle
fight-stage-help = Stage name, such as 1-7
fight-stage-long-help = { fight-stage-help }, supports all mainline stages, resource stages, annihilation, and some event stages.
    Leave blank to automatically select the current or last stage you fought. Use `maa activity` to view supported stages.

copilot-about = Auto-battle
copilot-uri-help = Local path or URI of the auto-battle job file, such as `1234.json` or `maa://1234`.

roguelike-about = Auto-integrate strategy (meat pigeon)
roguelike-theme-help = Theme of the integrated strategy, optional values are: Phantom, Mizuki, Sami
roguelike-theme-long-help = { roguelike-theme-help }.

    - Phantom: PHANTOM & CRIMSON SOLITAIRE;
    - Mizuki: MIZUKI & CAERULA ARBOR;
    - Sami: EXPEDITIONER'S JǪKLUMARKAR.

convert-about = Convert configuration file format, supports JSON, YAML, TOML
convert-input-help = Path of the input file
convert-output-help = Path of the output file, leave blank to output to standard output
convert-format-help = Output format, optional values are: json, yaml, toml, can be abbreviated as j, y, t
convert-format-long-help = { convert-format-help }.

    When specifying the path of the output file, the output format will be automatically inferred based on the file extension.
    When not specifying the path of the output file, the output format must be specified.
    When both are specified, the output file extension will be changed according to the specified format.

activity-about = View current game activity information
activity-client-help = Type of game client, optional values are: Official (default), YoStartEN, YoStartJP, YoStartKR, Txwy

list-about = List all available custom tasks
complete-about = Generate auto-completion script
complete-shell-help = Shell type of the generated auto-completion script, optional values are: bash, zsh, fish, powershell

## Installer output messages

fetching = Fetching the latest version information for { $name }, update channel: { $channel }
failed-fetch-version-json = Failed to retrieve version information file from { $url }
failed-parse-version-json = Failed to parse the version information file

unsupported-architecture = Unsupported architecture: { $arch }
unsupported-platform = Unsupported platform: { $arch } { $os }

asset-not-found = Package not found: { $name }

update-to-date = Already up to date: { $name } { $version }
failed-parse-version = Failed to parse the version number
found-newer-version = New version found: { $name } { $old } -> { $new }
package-cache-hit = Found installation package in cache: { $file }, skipping download
downloading = Downloading installation package: { $file }
installing = Installing: { $name } { $version }

skip-speed-test = Skipping image speed test, using the default address { $link }
testing-download-speed = Testing download speed
testing-mirror = Testing mirror: { $link }
found-faster-mirror = Found a faster mirror: { $link }
download-from-fastest-mirror = Downloading from the fastest mirror: { $link }

downloaded-verifying = Download completed, verifying
downloaded = Download completed
failed-download = Failed to download { $file }
verified = Verification successful
failed-verify = Verification failed

unsupported-archive = Unsupported compression format: { $file }
unknown-archive = Unknown compression format: { $file }
extracting = Extracting
extract = Extracting { $src } to { $dest }
skip-extract = Skipping file { $src }
extracted = Extraction completed
failed-extract = Failed to extract { $file }
failed-remove = Failed to remove { $file }
failed-symlink = Failed to create symlink { $file }
failed-create = Failed to create { $file }
failed-write = Failed to write { $file }
failed-set-permission = Failed to set permissions for { $file }
failed-read-entry = Failed to read file in the archive

create-dir = Creating directory { $dir }
remove-dir = Removing directory { $dir }

core-already-installed = MaaCore is already installed. Please use the `maa update` command to update MaaCore, or use the `maa install --force` command to force installation.
library-installed-by-other = MaaCore library installed at { $path } was found, but it was not installed by maa-cli and cannot be managed by maa-cli.
resource-installed-by-other = Installed resources found at { $path }, but not installed by maa-cli and cannot be managed by maa-cli.
deprecated-disable-library-option = The option to not install MaaCore library is deprecated, as it is not recommended to install MaaCore and resources separately. If you have a reason to do so, please submit an issue on GitHub.
deprecated-disable-resource-option = The option to not install MaaCore resources is deprecated, as it is not recommended to install MaaCore and resources separately. If you want to get the latest resources, you can use the `maa hot-update` command.
no-component-to-install = No MaaCore components need to be installed, skipping installation. The option to install only some MaaCore components is deprecated. If you have a reason to do so, please submit an issue on GitHub.

updating-resource-repository = Updating resource repository
cloning-resource-repository = Cloning resource repository
failed-open-resource-repository = Failed to open resource repository
failed-clone-resource-repository = Failed to clone resource repository
failed-pull-resource-repository = Failed to update resource repository
failed-find-remote = Remote repository not found: '{ $name }'
failed-find-reference = Reference not found: { $name }
failed-reference-to-annotated-commit = Failed to create a commit from reference { $name }
failed-merge-analysis = Merge analysis failed
failed-create-reference = Failed to create reference { $name }
failed-set-head = Failed to set HEAD
fast-forward-merge = Fast-forward merge
failed-merge = Merge failed
failed-checkout = Failed to checkout { $name }
repo-up-to-date = Repository is already up to date

## Start game messages
game-is-running = The game is running
start-game = Start the game: { $name }
stop-game = Stop the game: { $name }
game-ready = Game start completed
waiting-for-game = Waiting for the game to start
failed-connect-game = Failed to connect to the game

## Load maa-core messages
maa-core-already-loaded = MaaCore is already loaded
load-maa-core = Load MaaCore from { $path }
maa-core-not-found = MaaCore installation not found
use-system-maa-core = MaaCore not found, attempting to load from system path

## Load config messages
failed-load-config-skip = Failed to load configuration file { $path }, skipping, error message: { $error }
no-successful-config-found-use-default = No usable configuration file found, using default configuration

detected-client-type = Detected game client type: { $client }
detected-connection-type = Detected connection type: { $connection }

set-user-directory = Set user directory: { $path }
failed-set-user-directory = Failed to set user directory to { $path }

## Load resource messages
found-resource-directory = Found resource directory: { $path }
resource-directory-not-found = Resource directory not found
found-hot-update-resource = Found hot update resource directory: { $path }
hot-update-resource-not-exist = Hot update resource does not exist: { $path }
-globalize-resource = Non-CN server resource
use-globalize-resource = Loading { $path } for { -globalize-resource }
globalize-resource-twice-set = { -globalize-resource } { $loaded } already loaded, skipping { $path }
globalize-resource-not-found = { -globalize-resource } { $path } not found, skipping
-platform-diff-resource = Platform-specific resource
use-platform-diff-resource = Loading { $path } for { -platform-diff-resource }
platform-diff-resource-twice-set = { -platform-diff-resource } { $loaded } already loaded, skipping { $path }
platform-diff-resource-not-found = { -platform-diff-resource } { $path } not found, skipping
load-resource-from = Loading resources from { $path }
resource-not-exist = Resource directory { $path } does not exist, skipping loading

## Set static options messages
both-cpu-and-gpu-ocr-enabled = Both CPU and GPU OCR are enabled, CPU OCR will be disabled
use-cpu-ocr = Use CPU OCR
failed-use-cpu-ocr = Failed to use CPU OCR
use-gpu-ocr = Use GPU { $id } OCR
failed-use-gpu-ocr = Failed to use GPU { $id } OCR

## Set instance options messages
automatic-macplaytools = Automatically set touch mode to MacPlayTools
force-macplaytools = Connect to the game using PlayTools, force touch mode to MacPlayTools
force-disable-adb-lite = Connect to the game using PlayTools, force-disable ADB Lite
set-touch-mode = Set touch mode to { $mode }
failed-set-touch-mode = Failed to set touch mode to { $mode }
deploy-with-pause = { $enabled ->
    [true] Set pause game during deployment
    *[false] Do not pause game during deployment
}
failed-deploy-with-pause = { $enabled ->
    [true] Failed to set pause game during deployment
    *[false] Failed to set do not pause game during deployment
}
adb-lite = { $enabled ->
    [true] Enable ADB Lite
    *[false] Disable ADB Lite
}
failed-adb-lite = { $enabled ->
    [true] Failed to enable ADB Lite
    *[false] Failed to disable ADB Lite
}
kill-adb-on-exit = { $enabled ->
    [true] Set to close ADB on exit
    *[false] Set to not close ADB on exit
}
failed-kill-adb-on-exit = { $enabled ->
    [true] Failed to set close ADB on exit
    *[false] Failed to set not close ADB on exit
}

## Connection messages
connection-args-adb = Connect to { $device } using configuration { $config } (ADB: { $adb })
connection-args-playtools = Connect to { $address } using configuration { $config }

## Task messages
unknown-task-type = Unknown task type: { $task_type }. Please check if your task type is correct. If this is a new task type, not a spelling mistake, please submit an issue on GitHub.
task-type-startup = Startup
task-type-closedown = CloseDown
task-type-fight = Fight
task-type-recruit = Recruitment
task-type-infrast = Infrastructure
task-type-mall = Mall
task-type-award = Award
task-type-roguelike = Roguelike
task-type-copilot = Copilot
task-type-ssscopilot = SSS Copilot
task-type-reclamationalgorithm = Reclamation Algorithm
task-type-depot = Depot recognition
task-type-operbox = Operator Box Recognition
task-type-custom = Custom task
task-type-singlestep = Single-step task
task-type-videorecognition = Video recognition

failed-find-task-file = Task file not found: { $file }
task-directory-not-exist = Task directory does not exist: { $path }
append-task-with-param = Add task [{ $task }], parameters: { $params }
append-task-no-param = Add task [{ $task }]

## Copilot messages
failed-find-stage-file = Information file for stage { $stage } not found. Your resources may be outdated, please update resources.
failed-parse-stage-info = Failed to parse stage information, { $info }

copilot-cache-hit = Found auto-battle job file in cache: { $file }, skipping download
failed-download-copilot = Failed to download auto-battle job from { $url }
failed-parse-copilot = Failed to parse auto-battle job
copilot-downloaded = Downloaded auto-battle job from { $url }
failed-get-copilot-content = Failed to get auto-battle job content
failed-parse-copilot-content = Failed to parse auto-battle job content

failed-get-stage-name = Failed to get stage name
failed-get-group-name = Failed to get operator group name
failed-get-operator-name = Failed to get operator name

copilot-stage = Auto-battle stage:
copilot-operators = Operator list:

## Top-level messages
init-failed = MaaCore initialization failed
all-tasks-completed = All tasks completed
failed-process-message = Failed to process callback message, message code: { $code }, message content: { $message }

## Connection info messages
got-resolution = Successfully obtained resolution: { $width }x{ $height }
failed-get-resolution = Failed to obtain resolution
unsupported-resolution = Unsupported resolution ({ $width }x{ $height }): { $why }
low-screen-resolution = Low resolution ({ $width }x{ $height })
not-16-9 = Non 16:9 resolution ({ $width }x{ $height })

connected = Connected to { $address }
disconnected = Connection disconnected
reconnecting = Attempting { $times } reconnect
reconnected = Reconnection successful

failed-screencap = Failed to capture screenshot
fastest-way-screencap = Fastest screenshot method is { $method }, time taken: { $cost }ms
screencap-cost = Recent ten screenshot costs: { $min }ms ~ { $max }ms, average { $avg }ms

touch-mode-not-available = Touch mode not available
unknown-connection-info = Unknown connection information: { $message }

## Task chain messages
taskchain-start = Task chain { $name } started
taskchain-completed = Task chain { $name } completed
taskchain-stopped = Task chain { $name } stopped
taskchain-error = Task chain { $name } encountered an error

## Subtask error
failed-start-game = Failed to start the game
failed-auto-recruit = Failed to auto-recruit, { $why }
failed-recognize-drops = Failed to recognize dropped items
failed-report-penguinstats = Failed to report to PenguinStats, { $why }
failed-report-penguinstats-unknown-drops = Failed to report to PenguinStats, unknown dropped items
failed-report-penguinstats-unknown-drop-type = Failed to report to PenguinStats, unknown drop type
failed-report-yituliu = Failed to report to Yituliu, { $why }
failed-report-yituliu-unknown-drops = Failed to report to Yituliu, unknown dropped items
failed-report-yituliu-unknown-drop-type = Failed to report to Yituliu, unknown drop type
invalid-stage-for-recognition = Unable to recognize drops, { $why }
unknown-subtask-error = Unknown subtask error: { $message }

## Subtask start
game-offline = Game offline

## Mission
mission-start = Mission start
mission-start-times = Start mission { $times }
medicine-used = Use sanity medicine for the { $times } time
stone-used = Use originite prime for the { $times } time

prts-error = Proxy battle error, abort this battle

## Recruit
recruit-refresh = Refresh recruitment tags
recruit-confirm = Confirm recruitment

## Infrastructure
infrast-dorm-double-confirm = Operator conflict in dorm

## RogueLike
roguelike-start = Start exploration { $times }
roguelike-abandon = Abandon this exploration
roguelike-complete = Exploration completed, cleared!

invest = Invest for the { $times } time
invest-full = Investment reaches the upper limit, unable to continue investing
special-item-bought = Purchased special items!

mission-complete = Mission completed
mission-failed = Mission failed
trader-enter = Enter the store
safe-house-enter = Enter the safe house
normal-dps-enter = Enter the normal battle
emergency-dps-enter = Enter the emergency battle
dreadful-foe-enter = Enter the treacherous path

unknown-subtask-start = Unknown subtask started: { $message }

## Subtask extra info
depot-recognition = Depot recognition result: { $result }
operator-recognition = Operator recognition result: { $result }

drops = Drops in this battle: { $drops }
sanity-before-stage = Current sanity: { $sanity } / { $max }

facility-enter = Enter the facility: { $facility }#{ $index }
product-of-facility = Product of facility { $facility }#{ $index }: { $product }
product-incorrect = Incorrect product for facility { $facility }#{ $index }: { $product }
product-changed = Product changed for facility { $facility }#{ $index }: { $product }

not-enough-staff = Not enough operators in facility { $facility }#{ $index }

custom-infrast-operators = Operators in facility { $facility }#{ $index }: { $operators }
custom-infrast-candidates = Candidate operators in facility { $facility }#{ $index }: { $candidates }
custom-infrast-both = Operators in facility { $facility }#{ $index }: { $operators }, Candidate operators: { $candidates }

## Facility
Control = Control Center
Mfg = Manufacturing Station
Trade = Trading Station
Power = Power Station
Office = Office
Reception = Reception Room
Dorm = Dormitory
Processing = Processing Station
Training = Training Room
UnknownFacility = Unknown Facility

## Product
LMD = LMD
PureGold = Pure Gold
Orundum = Orundum
OriginiumShard = Originium Shard
Dualchip = Dualchip
BattleRecord = Battle Record
UnknownProduct = Unknown Product

## Recruit
recruit-tags = Discovered { $star } star tags: { $tags }
recruit-special-tag = Discovered special tag: { $tag }
recruit-robot-tag = Discovered robot tag: { $tag }
recruit-tags-selected = Selected tags: { $tags }
recruit-no-permit = Insufficient recruitment permits, unable to recruit

## RogueLike
roguelike-stage-enter = Enter stage: { $name }
roguelike-stage-info-error = Stage recognition error
roguelike-event = Discovered event: { $name }

roguelike-pass = Strategy integration passed
roguelike-fail = Strategy integration failed
roguelike-settlement = Difficulty { $difficulty } { $pass }:
    Explored { $explore } layers, moved forward { $steps } steps,
    Normal battles { $combat } times, elite battles { $emergency } times, boss battles { $boss } times,
    Recruited { $recruit } times, collected { $object } treasures,
    Scored { $score }, gained { $exp } experience points, gained { $skill } skill points.

## Copilot
Copilot = Copilot
SSSCopilot = SSS Copilot

battle-formation = Formation: { $formation }
battle-formation-selected = Selected operators: { $selected }
current-copilot-action = Current auto-battle action: { $action } { $target } { $doc }
unsupported-level = Unsupported stage, please check the stage name!

Deploy = Deploy
UseSkill = Use skill
Retreat = Retreat
SwitchSpeed = Double speed
BulletTime = Bullet time
SkillUsage = Skill usage
Output = Output
SkillDaemon = Set up auto-battle
MoveCamera = Move camera
DrawCard = Draw card
CheckIfStartOver = Check if start over

## SSS
sss-stage-enter = Enter stage: { $name }
sss-settlement = Squad maintenance settlement: { $why }
sss-game-pass = Squad maintenance passed

unknown-subtask-extra-info = Unknown subtask information: { $message }

## Task Summary
task-summary = Task summary

task-state-unstarted = Not started
task-state-unfinished = Started but not completed
task-state-completed = Completed
task-state-stopped = Stopped
task-state-error = Error

summary-infrast-operator = Infrast operators: { $operators }
summary-infrast-candidate = Infrast candidate operators: { $candidates }
summary-infrast-both = Infrast operators: { $operators }, Candidate operators: { $candidates }

summary-fight-stage = Grind { $stage }
summary-fight-times = { $times } times
summary-fight-medicine = , use { $medicine } bottles of sanity medicine
summary-fight-stone = , spend { $stone } originite primes
summary-fight-drop = , drop records are as follows:
summary-fight-total-drop = Total drops:

recruit-refreshed = Refreshed
recruit-recruited = Recruited
recruit-refreshed-times = Refreshed { $times } times
recruit-recruited-times = Recruited { $times } times
recruit-tags-records = Tag records:

roguelike-explore-times = Explore { $times } times
roguelike-invest-times = Invest { $times } originium primes

## Common error messages
## Network error messages
failed-create-reqwest-client = Failed to create reqwest client
failed-send-request = Failed to send request to { $url }
failed-response-status = Request failed, status code: { $status }

## File error messages
failed-open-file = Failed to open file { $file }
failed-read-file = Failed to read file { $file }
failed-write-file = Failed to write file { $file }

## Json error messages
failed-deserialize-json = Failed to deserialize JSON
failed-serialize-json = Failed to serialize JSON
unknown-value = Unknown value: { $value }
value-type-mismatch = Value { $value } must be { $expected }

## Convert error messages
invalid-utf8-path = Path contains invalid UTF-8 characters

## Async error messages
failed-create-tokio-runtime = Failed to create Tokio runtime
failed-register-signal-handler = Failed to register signal handler
interrupted = User interrupted

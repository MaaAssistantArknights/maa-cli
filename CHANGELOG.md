# Change Log

## Release 0.4.3

### Features

- Add `preset` field for connection configuration by @wangl-cc in [#195](https://github.com/MaaAssistantArknights/maa-cli/pull/195)
- Add `client` field to `Weekday` condition used to adjust date by @wangl-cc in [#203](https://github.com/MaaAssistantArknights/maa-cli/pull/203)

### Bug Fixes

- Add newline to summary detail of roguelike by @wangl-cc in [#194](https://github.com/MaaAssistantArknights/maa-cli/pull/194)
- Use 32 bit int and float in `MAAValue` by @wangl-cc in [#198](https://github.com/MaaAssistantArknights/maa-cli/pull/198)

### Documentation

- Fix format of toml example by @wangl-cc

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.2...v0.4.3>

## Release 0.4.2

### Features

- Add condition `DayMod` for multi-day plan by @wangl-cc in [#190](https://github.com/MaaAssistantArknights/maa-cli/pull/190)

### Bug Fixes

- If start time is later than end, treat it as crossing midnight by @wangl-cc in [#189](https://github.com/MaaAssistantArknights/maa-cli/pull/189)

### Miscellaneous

- Add condition `DayMod` for task schema by @wangl-cc

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.1...v0.4.2>

## Release 0.4.1

### Performance

- Use `Cow` to avoid unnecessary allocation by @wangl-cc in [#181](https://github.com/MaaAssistantArknights/maa-cli/pull/181)

### Documentation

- Mention that partial installation of MaaCore is not recommended by @wangl-cc

### Miscellaneous

- Fix typos by @wangl-cc in [#179](https://github.com/MaaAssistantArknights/maa-cli/pull/179)
- Rename `as_string` to `as_str` by @wangl-cc in [#182](https://github.com/MaaAssistantArknights/maa-cli/pull/182)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.0...v0.4.1>

## Release 0.4.0

### Features

- Search both origin and canonicalized directory of `current_exe` by @wangl-cc in [#94](https://github.com/MaaAssistantArknights/maa-cli/pull/94)
- Add a new subcommand `fight` by @wangl-cc in [#104](https://github.com/MaaAssistantArknights/maa-cli/pull/104)
- Add `BoolInput` to query user for boolean input by @wangl-cc in [#107](https://github.com/MaaAssistantArknights/maa-cli/pull/107)
- Qurey `start_game_enabled` and `client_type` in startup task by @wangl-cc in [#110](https://github.com/MaaAssistantArknights/maa-cli/pull/110)
- Add subcommand `copilot` to complete the auto-battle feature  by @hzxjy1 in [#127](https://github.com/MaaAssistantArknights/maa-cli/pull/127)
- **BREAKING**:Resource update and refactor maa core binding by @wangl-cc in [#126](https://github.com/MaaAssistantArknights/maa-cli/pull/126)
- **BREAKING**:Download native binaries instead of universal binaries on macOS by @wangl-cc in [#133](https://github.com/MaaAssistantArknights/maa-cli/pull/133)
- Add stage argument to fight task by @wangl-cc in [#134](https://github.com/MaaAssistantArknights/maa-cli/pull/134)
- Subcommand `roguelike` by @wangl-cc in [#136](https://github.com/MaaAssistantArknights/maa-cli/pull/136)
- Don't run set options test in CI by @wangl-cc in [#143](https://github.com/MaaAssistantArknights/maa-cli/pull/143)
- Auto set remote url based on locale by @wangl-cc in [#141](https://github.com/MaaAssistantArknights/maa-cli/pull/141)
- Add alias for component and update fish completion by @wangl-cc in [#149](https://github.com/MaaAssistantArknights/maa-cli/pull/149)
- **BREAKING**:Launch PlayCover App only on macOS by @wangl-cc in [#152](https://github.com/MaaAssistantArknights/maa-cli/pull/152)
- **BREAKING**:Log with `env_logger` and show task summary when stopped by @wangl-cc in [#153](https://github.com/MaaAssistantArknights/maa-cli/pull/153)
- Add name field to task config, use it in summary by @wangl-cc in [#154](https://github.com/MaaAssistantArknights/maa-cli/pull/154)
- Add `convert` subcommand to convert config file to another format by @wangl-cc in [#156](https://github.com/MaaAssistantArknights/maa-cli/pull/156)
- Read stage activity from StageActivity.json by @wangl-cc in [#159](https://github.com/MaaAssistantArknights/maa-cli/pull/159)
- Add boolean conditions by @wangl-cc in [#161](https://github.com/MaaAssistantArknights/maa-cli/pull/161)
- Better input by @wangl-cc in [#163](https://github.com/MaaAssistantArknights/maa-cli/pull/163)
- Exit with error when taskchain error by @wangl-cc in [#169](https://github.com/MaaAssistantArknights/maa-cli/pull/169)
- **BREAKING**:Return the error when loading SharedLibrary fail by @wangl-cc in [#172](https://github.com/MaaAssistantArknights/maa-cli/pull/172)
- **BREAKING**:Split startup and closedown from fight by @wangl-cc in [#174](https://github.com/MaaAssistantArknights/maa-cli/pull/174)

### Bug Fixes

- Log message by @wangl-cc
- Only open playcover app when using playtools by @wangl-cc in [#137](https://github.com/MaaAssistantArknights/maa-cli/pull/137)
- Dry-run not working by @wangl-cc in [#140](https://github.com/MaaAssistantArknights/maa-cli/pull/140)
- **BREAKING**:Ensure extra share name is a name instead of a path by @wangl-cc in [#160](https://github.com/MaaAssistantArknights/maa-cli/pull/160)

### Refactor

- Use `object!` macro to create `Value::Object` by @wangl-cc in [#105](https://github.com/MaaAssistantArknights/maa-cli/pull/105)
- Rename `TaskList` to `TaskConfig` and add methods by @wangl-cc in [#108](https://github.com/MaaAssistantArknights/maa-cli/pull/108)
- Move common args of `run` in struct `CommonArgs` by @wangl-cc in [#109](https://github.com/MaaAssistantArknights/maa-cli/pull/109)
- Add `Task::new_with_default()` to simplify code by @wangl-cc in [#111](https://github.com/MaaAssistantArknights/maa-cli/pull/111)
- **BREAKING**:Core and cli installer by @wangl-cc in [#118](https://github.com/MaaAssistantArknights/maa-cli/pull/118)
- Rename Value to MAAValue by @wangl-cc
- Detect game ready and close game by TCP connection by @wangl-cc in [#164](https://github.com/MaaAssistantArknights/maa-cli/pull/164)
- Rename `MAATask` to `TaskType` and move to `maa-sys` by @wangl-cc in [#173](https://github.com/MaaAssistantArknights/maa-cli/pull/173)

### Documentation

- Add build options and update usage and config by @wangl-cc in [#132](https://github.com/MaaAssistantArknights/maa-cli/pull/132)
- Correct zh-CN document link by @hzxjy1 in [#171](https://github.com/MaaAssistantArknights/maa-cli/pull/171)

### Testing

- Fix test failure on CI caused by create user resource dir by @wangl-cc in [#142](https://github.com/MaaAssistantArknights/maa-cli/pull/142)
- Ignore tests that attempt to create a directory in user space by @wangl-cc in [#144](https://github.com/MaaAssistantArknights/maa-cli/pull/144)

### Miscellaneous

- Fix typos by @wangl-cc
- Remove debug print by @wangl-cc
- Group all non breaking updates into a single PR by @wangl-cc in [#113](https://github.com/MaaAssistantArknights/maa-cli/pull/113)
- Only bump `Cargo.lock` with dependabot by @wangl-cc in [#116](https://github.com/MaaAssistantArknights/maa-cli/pull/116)
- Change copilot input prompt by @wangl-cc in [#135](https://github.com/MaaAssistantArknights/maa-cli/pull/135)
- **BREAKING**:Add JSON schemas and change file structure by @wangl-cc in [#157](https://github.com/MaaAssistantArknights/maa-cli/pull/157)
- Update dependencies by @wangl-cc
- Update `windows-sys` to `windows` by @wangl-cc in [#170](https://github.com/MaaAssistantArknights/maa-cli/pull/170)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.12...v0.4.0>

## Release 0.3.12

### Features

- Load `MaaCore` with name if core dir not found by @wangl-cc in [#70](https://github.com/MaaAssistantArknights/maa-cli/pull/70)
- Add `user_resource` option in asst config by @wangl-cc in [#72](https://github.com/MaaAssistantArknights/maa-cli/pull/72)
- Make log level related options global by @wangl-cc in [#73](https://github.com/MaaAssistantArknights/maa-cli/pull/73)
- Add `--dry-run` option to `run` command by @wangl-cc in [#76](https://github.com/MaaAssistantArknights/maa-cli/pull/76)
- Support Windows by @wangl-cc in [#77](https://github.com/MaaAssistantArknights/maa-cli/pull/77)
- Better error message when directory not found by @wangl-cc
- Add support for static options by @wangl-cc in [#88](https://github.com/MaaAssistantArknights/maa-cli/pull/88)

### Bug Fixes

- Canonicalize returned path of `current_exe` by @wangl-cc in [#71](https://github.com/MaaAssistantArknights/maa-cli/pull/71)
- `user_resource` should be a flag instead of an option by @wangl-cc in [#74](https://github.com/MaaAssistantArknights/maa-cli/pull/74)
- Load client resource when playtools is not true by @wangl-cc in [#75](https://github.com/MaaAssistantArknights/maa-cli/pull/75)
- Failed to exit on windows by @wangl-cc in [#79](https://github.com/MaaAssistantArknights/maa-cli/pull/79)
- `current_exe` on windows and all platform without `self` feature by @wangl-cc in [#78](https://github.com/MaaAssistantArknights/maa-cli/pull/78)

### Documentation

- Remove outdated comment by @wangl-cc
- Update README to match the latest version by @wangl-cc

### Testing

- Add tests for `ClientType` and fix typo by @wangl-cc in [#85](https://github.com/MaaAssistantArknights/maa-cli/pull/85)

### Miscellaneous

- Add cliff.toml to generate changelog when release by @wangl-cc
- Add some metadata to Cargo.toml by @wangl-cc

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.11...v0.3.12>

## Release 0.3.11

### Bug Fixes

- Make `Array` higher priority than `Input*` in `Value` by @wangl-cc

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.10...v0.3.11>

## Release 0.3.10

### Features

- Allow user input in task definition by @wangl-cc in [#54](https://github.com/MaaAssistantArknights/maa-cli/pull/54)
- Add option `strategy`  for  variant by @wangl-cc in [#64](https://github.com/MaaAssistantArknights/maa-cli/pull/64)

### Documentation

- Change "MaaTouch" to "MAATouch" by @hzxjy1 in [#53](https://github.com/MaaAssistantArknights/maa-cli/pull/53)

### Testing

- Add test for deserializing value with input by @wangl-cc

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.9...v0.3.10>

## Release 0.3.9

### Features

- `MAA_EXTRA_SHARE_NAME` to specify extra share name at compile time by @wangl-cc in [#43](https://github.com/MaaAssistantArknights/maa-cli/pull/43)
- Add feature `self` to disable self update by disable it by @wangl-cc in [#44](https://github.com/MaaAssistantArknights/maa-cli/pull/44)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.8...v0.3.9>

## Release 0.3.8

### Bug Fixes

- Don't clear resource dir if `no_resource` is true by @wangl-cc in [#41](https://github.com/MaaAssistantArknights/maa-cli/pull/41)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.7...v0.3.8>

## Release 0.3.7

### Features

- Add `core` field to CLI configuration by @wangl-cc in [#38](https://github.com/MaaAssistantArknights/maa-cli/pull/38)
- Add completion by @wangl-cc in [#39](https://github.com/MaaAssistantArknights/maa-cli/pull/39)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.6...v0.3.7>

## Release 0.3.6

### Features

- Import download by @wangl-cc in [#36](https://github.com/MaaAssistantArknights/maa-cli/pull/36)

### Bug Fixes

- Handle symlink when extract zip file by @wangl-cc in [#37](https://github.com/MaaAssistantArknights/maa-cli/pull/37)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.5...v0.3.6>

## Release 0.3.5

### Features

- Load resources based on `cilent_type` and if use `PlayTools` by @wangl-cc in [#33](https://github.com/MaaAssistantArknights/maa-cli/pull/33)

### Bug Fixes

- Don't skip file with same size by @wangl-cc in [#34](https://github.com/MaaAssistantArknights/maa-cli/pull/34)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.4...v0.3.5>

## Release 0.3.4

### Features

- Add CLI config file by @wangl-cc in [#31](https://github.com/MaaAssistantArknights/maa-cli/pull/31)

### Testing

- Fix config path on linux by @wangl-cc

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.3...v0.3.4>

## Release 0.3.3

### Features

- Drop assistant on Ctrl+C by @horror-proton in [#29](https://github.com/MaaAssistantArknights/maa-cli/pull/29)

### Bug Fixes

- Don't ensure `lib_dir` clean by @wangl-cc in [#30](https://github.com/MaaAssistantArknights/maa-cli/pull/30)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.2...v0.3.3>

## Release 0.3.2

### Bug Fixes

- Version parsing of MaaCore by @wangl-cc

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.1...v0.3.2>

## Release 0.3.1

### Features

- Cross compile for arm64 linux by @wangl-cc in [#25](https://github.com/MaaAssistantArknights/maa-cli/pull/25)

### Documentation

- Update README by @wangl-cc
- 中文作为主README，英文作为README-EN.md by @wangl-cc
- Update badge by @wangl-cc

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.0...v0.3.1>

## Release 0.3.0

### Features

- Better maa callback based on MacGUI by @wangl-cc
- Print lib and resource dir by @wangl-cc
- **BREAKING**:Rename touch modes and default to ADB by @wangl-cc
- **BREAKING**:Support PlayCover connection by @wangl-cc
- Install package with `maa-updater` by @wangl-cc
- Speed test from mirrors and fix for linux by @wangl-cc
- Improve speed test of updater by @wangl-cc
- Make all filename parameters relative to config directory by @wangl-cc
- Improve start and close game of playcover mode by @wangl-cc
- Extract for windows by @wangl-cc
- **BREAKING**:Maa-updater can only install from prebuilt package by @wangl-cc
- Download from GitHub release in CI by @wangl-cc
- **BREAKING**:Remove mod type by @wangl-cc
- Imporve build script by @wangl-cc
- **BREAKING**:Drop support for windows and fix test on macOS by @wangl-cc in [#5](https://github.com/MaaAssistantArknights/maa-cli/pull/5)
- **BREAKING**:Maa-run as a subcommand of maa-helper to set env vars for maa-run by @wangl-cc in [#7](https://github.com/MaaAssistantArknights/maa-cli/pull/7)
- **BREAKING**:Import help and version will only show MaaCore version by @wangl-cc in [#12](https://github.com/MaaAssistantArknights/maa-cli/pull/12)
- **BREAKING**:More useful `maa` command by @wangl-cc in [#13](https://github.com/MaaAssistantArknights/maa-cli/pull/13)
- Add yaml support for config file by @wangl-cc in [#14](https://github.com/MaaAssistantArknights/maa-cli/pull/14)
- Add a field `resources` to specify additional resource files by @wangl-cc in [#15](https://github.com/MaaAssistantArknights/maa-cli/pull/15)
- Impl ToCString for both PathBuf and &PathBuf by @wangl-cc
- Better log system and message handling by @wangl-cc in [#17](https://github.com/MaaAssistantArknights/maa-cli/pull/17)
- Failed message will be printed to debug log by @wangl-cc in [#18](https://github.com/MaaAssistantArknights/maa-cli/pull/18)
- Support absolute path for additional resource by @wangl-cc in [#19](https://github.com/MaaAssistantArknights/maa-cli/pull/19)
- New option `--user-resource` by @wangl-cc

### Bug Fixes

- Typo by @wangl-cc
- Extract on linux by @wangl-cc
- Extract on linux by @wangl-cc
- Regex to match python files by @wangl-cc
- Don't use default features of chrono by @wangl-cc
- Check if file is dir instead of outpath by @wangl-cc
- Asset_name for windows by @wangl-cc
- Message handling by @wangl-cc in [#6](https://github.com/MaaAssistantArknights/maa-cli/pull/6)
- Wrong match in get_asset by @wangl-cc
- Name of `maa-cli` should be `maa` by @wangl-cc
- Download url with `MAA_CLI_DOWNLOAD` by @wangl-cc
- Remove  duplicate log for additional resource by @wangl-cc
- Don't treat other error as file not found during parse asst_config by @wangl-cc
- Yaml support by @wangl-cc

### Refactor

- Better error handle by @wangl-cc
- **BREAKING**:Remove maa-util, split maa-sys and other imporves by @wangl-cc
- **BREAKING**:Rename `maa-runner` to `maa-cli` by @wangl-cc
- **BREAKING**:Rename workspace members to avoid confusion by @wangl-cc in [#9](https://github.com/MaaAssistantArknights/maa-cli/pull/9)
- **BREAKING**:Remove maa-run and use dlopen to load MaaCore by @wangl-cc in [#24](https://github.com/MaaAssistantArknights/maa-cli/pull/24)

### Documentation

- Add installation and usage by @wangl-cc
- Add README-CN by @wangl-cc
- Add feature and update todo by @wangl-cc
- Add CHANGELOG of maa-cli by @wangl-cc
- Clean CHANGELOG by @wangl-cc
- Update README by @wangl-cc
- Add doc about log level by @wangl-cc
- Add note for `adb` by @wangl-cc
- Add notice for `adb` by @wangl-cc
- Doc about create config dir by @wangl-cc
- Explain which binary to download for macOS and Linux by @wangl-cc

### Testing

- Add AsstConfig test and fix get_dir test by @wangl-cc

### Miscellaneous

- Lint by clippy and fmt by @wangl-cc
- Apply clippy by @wangl-cc
- Move example to config_exmaples to avoid confusion by @wangl-cc
- Change package name to maa-cli by @wangl-cc
- Update comment by @wangl-cc

<!-- generated by git-cliff -->

# Release Notes

## Release 0.5.7

### Features

- Add roguelike theme `JieGarden` by [@wangl-cc](https://github.com/wangl-cc) in [#411](https://github.com/MaaAssistantArknights/maa-cli/pull/411)

### Bug Fixes

- Use `txwy` instead of `Txwy` when init by [@PFiS1737](https://github.com/PFiS1737) in [#410](https://github.com/MaaAssistantArknights/maa-cli/pull/410)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.5.6...v0.5.7>

## Release 0.5.6

### Features

- Add AsstBackToHome and AsstSetConnectionExtras bindings by [@YinY1](https://github.com/YinY1) in [#404](https://github.com/MaaAssistantArknights/maa-cli/pull/404)

### Documentation

- Update DEVELOPMENT.md with contribution guidelines by [@wangl-cc](https://github.com/wangl-cc) in [#401](https://github.com/MaaAssistantArknights/maa-cli/pull/401)

### Testing

- Improve test coverage by [@wangl-cc](https://github.com/wangl-cc) in [#409](https://github.com/MaaAssistantArknights/maa-cli/pull/409)

### Miscellaneous

- Ota.maa.plus -> api.maa.plus by [@MistEO](https://github.com/MistEO)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.5.5...v0.5.6>

## Release 0.5.5

### Features

- Add `load` related methods for Assistant safe API by [@wangl-cc](https://github.com/wangl-cc) in [#383](https://github.com/MaaAssistantArknights/maa-cli/pull/383)
- 为Waydroid实现ExternalApp by [@Jackhr-arch](https://github.com/Jackhr-arch) in [#388](https://github.com/MaaAssistantArknights/maa-cli/pull/388)
- Switch reqwest to use native-tls by [@wangl-cc](https://github.com/wangl-cc) in [#396](https://github.com/MaaAssistantArknights/maa-cli/pull/396)
- Adapt recent update of fight series by [@wangl-cc](https://github.com/wangl-cc) in [#392](https://github.com/MaaAssistantArknights/maa-cli/pull/392)

### Documentation

- 在 install.md 中添加 windows 依赖库安装指引 by [@LushMoon](https://github.com/LushMoon) in [#387](https://github.com/MaaAssistantArknights/maa-cli/pull/387)
- Note about limited Windows support by [@wangl-cc](https://github.com/wangl-cc)
- Add development docs by [@wangl-cc](https://github.com/wangl-cc) in [#400](https://github.com/MaaAssistantArknights/maa-cli/pull/400)

### Testing

- Switch to cargo-llvm-cov for coverage by [@wangl-cc](https://github.com/wangl-cc) in [#393](https://github.com/MaaAssistantArknights/maa-cli/pull/393)

### Security

- Bump ring from 0.17.8 to 0.17.13 by [@dependabot](https://github.com/dependabot) in [#382](https://github.com/MaaAssistantArknights/maa-cli/pull/382)
- Bump tokio from 1.43.0 to 1.44.2 by [@dependabot](https://github.com/dependabot) in [#390](https://github.com/MaaAssistantArknights/maa-cli/pull/390)

### Miscellaneous

- Update dependencies and migrate to Rust 2024 edition by [@wangl-cc](https://github.com/wangl-cc) in [#395](https://github.com/MaaAssistantArknights/maa-cli/pull/395)
- Update vendored-openssl feature to include reqwest by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.5.4...v0.5.5>

## Release 0.5.4

### Features

- Add support for starting with seed in Sarkaz theme and mode 1 by [@wangl-cc](https://github.com/wangl-cc) in [#368](https://github.com/MaaAssistantArknights/maa-cli/pull/368)
- Add `warn_on_update_failure` option for hot update by [@wangl-cc](https://github.com/wangl-cc) in [#378](https://github.com/MaaAssistantArknights/maa-cli/pull/378)

### Bug Fixes

- Use self-replace to update maa-cli self by [@wangl-cc](https://github.com/wangl-cc) in [#363](https://github.com/MaaAssistantArknights/maa-cli/pull/363)

### Documentation

- Remove outdated information about `fastdeploy` by [@wangl-cc](https://github.com/wangl-cc)
- Fix nix MaaCore installation by [@wangl-cc](https://github.com/wangl-cc)

### Testing

- Retry for more tests by [@wangl-cc](https://github.com/wangl-cc) in [#360](https://github.com/MaaAssistantArknights/maa-cli/pull/360)
- Use mock environment variables in maa-dirs tests by [@wangl-cc](https://github.com/wangl-cc) in [#366](https://github.com/MaaAssistantArknights/maa-cli/pull/366)

### Miscellaneous

- Fix moved url in schemas by [@wangl-cc](https://github.com/wangl-cc)
- Fix typo by [@wangl-cc](https://github.com/wangl-cc)
- Fix new lint errors by [@wangl-cc](https://github.com/wangl-cc)
- Simplify some code by [@wangl-cc](https://github.com/wangl-cc)
- Use lld as default linker on windows by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.5.3...v0.5.4>

## Release 0.5.3

### Bug Fixes

- Use ubuntu-22.04 instead of ubuntu-latest for build jobs by [@wangl-cc](https://github.com/wangl-cc) in [#359](https://github.com/MaaAssistantArknights/maa-cli/pull/359)

### Refactor

- Move all packages to `crates` subdir by [@wangl-cc](https://github.com/wangl-cc) in [#356](https://github.com/MaaAssistantArknights/maa-cli/pull/356)

### Miscellaneous

- Fix generated links by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.5.2...v0.5.3>

## Release 0.5.2

### Features

- Add serialize for TouchMode and TaskType enum by [@wangl-cc](https://github.com/wangl-cc)
- Improve the error handling of assistant API by [@wangl-cc](https://github.com/wangl-cc) in [#346](https://github.com/MaaAssistantArknights/maa-cli/pull/346)
- Add difficulty parameter to roguelike preset by [@wangl-cc](https://github.com/wangl-cc) in [#351](https://github.com/MaaAssistantArknights/maa-cli/pull/351)
- Support multiple tools to craft in reclamation by [@wangl-cc](https://github.com/wangl-cc) in [#353](https://github.com/MaaAssistantArknights/maa-cli/pull/353)
- Add support for Windows aarch64 by [@wangl-cc](https://github.com/wangl-cc) in [#354](https://github.com/MaaAssistantArknights/maa-cli/pull/354)

### Refactor

- Split shared types into maa-types crate by [@wangl-cc](https://github.com/wangl-cc) in [#341](https://github.com/MaaAssistantArknights/maa-cli/pull/341)
- Replace OnceLock with LazyLock by [@wangl-cc](https://github.com/wangl-cc)

### Testing

- Simplify to_cstring test by [@wangl-cc](https://github.com/wangl-cc)
- Improve test about maa-sys::Error by [@wangl-cc](https://github.com/wangl-cc) in [#347](https://github.com/MaaAssistantArknights/maa-cli/pull/347)
- Retry 3 times for test that downloads files from internet by [@wangl-cc](https://github.com/wangl-cc) in [#348](https://github.com/MaaAssistantArknights/maa-cli/pull/348)

### Miscellaneous

- Centralize and update dependencies by [@wangl-cc](https://github.com/wangl-cc) in [#342](https://github.com/MaaAssistantArknights/maa-cli/pull/342)
- Replace some allow with expect and add reason by [@wangl-cc](https://github.com/wangl-cc)
- Display licenses of third party projects by [@wangl-cc](https://github.com/wangl-cc)
- Ignore licenses.html in typos check by [@wangl-cc](https://github.com/wangl-cc)
- Add editorconfig by [@wangl-cc](https://github.com/wangl-cc)
- Remove unnecessary lifetime annotations by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.5.1...v0.5.2>

## Release 0.5.0

### Features

- **BREAKING**: Remove deprecated cleanup targets and improve tests by [@wangl-cc](https://github.com/wangl-cc) in [#315](https://github.com/MaaAssistantArknights/maa-cli/pull/315)
- **BREAKING**: Add `reclamation` and improve `fight`, `roguelike`, `copilot` commands by [@wangl-cc](https://github.com/wangl-cc) in [#313](https://github.com/MaaAssistantArknights/maa-cli/pull/313)
- Add support for encrypted ssh key and ssh-agent by [@wangl-cc](https://github.com/wangl-cc) in [#337](https://github.com/MaaAssistantArknights/maa-cli/pull/337)

### Bug Fixes

- Retry `remove_dir_all` 3 times in `ensure_clean` by [@wangl-cc](https://github.com/wangl-cc) in [#320](https://github.com/MaaAssistantArknights/maa-cli/pull/320)
- Create a owned copy of version string of MaaCore instead of a reference by [@wangl-cc](https://github.com/wangl-cc) in [#327](https://github.com/MaaAssistantArknights/maa-cli/pull/327)
- Task name of reclamation algorithm by [@wangl-cc](https://github.com/wangl-cc) in [#336](https://github.com/MaaAssistantArknights/maa-cli/pull/336)

### Performance

- Use manual Debug implementation and remove deprecated AsRef by [@wangl-cc](https://github.com/wangl-cc) in [#330](https://github.com/MaaAssistantArknights/maa-cli/pull/330)

### Refactor

- **BREAKING**: Switch from `native-tls` to `rustls` by [@wangl-cc](https://github.com/wangl-cc) in [#317](https://github.com/MaaAssistantArknights/maa-cli/pull/317)
- Split `dirs` module into a separate crate by [@wangl-cc](https://github.com/wangl-cc) in [#321](https://github.com/MaaAssistantArknights/maa-cli/pull/321)
- Use RwLock instead of TLS to store shared library by [@wangl-cc](https://github.com/wangl-cc) in [#328](https://github.com/MaaAssistantArknights/maa-cli/pull/328)
- Use windows-sys to replace windows crate by [@wangl-cc](https://github.com/wangl-cc) in [#331](https://github.com/MaaAssistantArknights/maa-cli/pull/331)

### Documentation

- Fix typo by [@wangl-cc](https://github.com/wangl-cc)
- Improve docs about fetching via SSH by [@wangl-cc](https://github.com/wangl-cc)

### Testing

- **BREAKING**: Run tests even some environment variables are not set by [@wangl-cc](https://github.com/wangl-cc) in [#316](https://github.com/MaaAssistantArknights/maa-cli/pull/316)
- Fix env var name and logic in version test by [@wangl-cc](https://github.com/wangl-cc)
- Ignore data_relative test by [@wangl-cc](https://github.com/wangl-cc)

### Miscellaneous

- Sort dependencies by name by [@wangl-cc](https://github.com/wangl-cc)
- Remove unnecessary reference by [@wangl-cc](https://github.com/wangl-cc)
- Update schema ssh configuration by [@wangl-cc](https://github.com/wangl-cc)
- Remove [bot] suffix from username in changelog by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.12...v0.5.0>

## Release 0.4.12

### Features

- Use `Official` as the default value for `client_type` by [@wangl-cc](https://github.com/wangl-cc) in [#312](https://github.com/MaaAssistantArknights/maa-cli/pull/312)

### Bug Fixes

- Handle `ConnectFailed` message in callback by [@wangl-cc](https://github.com/wangl-cc)

### Performance

- Slightly decrease allocations in processing tasks by [@wangl-cc](https://github.com/wangl-cc)

### Documentation

- Remove outdated FAQ entries by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.11...v0.4.12>

## Release 0.4.11

### Features

- Set `client_type` for `Fight` and `Closedown` automatically by [@wangl-cc](https://github.com/wangl-cc) in [#305](https://github.com/MaaAssistantArknights/maa-cli/pull/305)
- Support client type for `Closedown` command by [@wangl-cc](https://github.com/wangl-cc) in [#309](https://github.com/MaaAssistantArknights/maa-cli/pull/309)

### Bug Fixes

- Remove resource mirror by [@wangl-cc](https://github.com/wangl-cc) in [#311](https://github.com/MaaAssistantArknights/maa-cli/pull/311)

### Refactor

- Generalize external app open/close by [@wangl-cc](https://github.com/wangl-cc) in [#308](https://github.com/MaaAssistantArknights/maa-cli/pull/308)

### Documentation

- Unified case of `MaaTouch` by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.10...v0.4.11>

## Release 0.4.10

### Features

- Improve error message when failing to add task by [@wangl-cc](https://github.com/wangl-cc)

### Bug Fixes

- Correctly handle the return value of `AsstAppendTask` by [@wangl-cc](https://github.com/wangl-cc) in [#300](https://github.com/MaaAssistantArknights/maa-cli/pull/300)

### Refactor

- Parse and convert of `TouchMode` and `TaskType` by [@wangl-cc](https://github.com/wangl-cc) in [#303](https://github.com/MaaAssistantArknights/maa-cli/pull/303)

### Documentation

- Update runtime loading section of maa-sys by [@wangl-cc](https://github.com/wangl-cc)

### Testing

- Remove `MAA_*_DIR` env vars before test by [@wangl-cc](https://github.com/wangl-cc) in [#302](https://github.com/MaaAssistantArknights/maa-cli/pull/302)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.9...v0.4.10>

## Release 0.4.9

### Features

- Support Sarkaz rogue by [@hguandl](https://github.com/hguandl) in [#293](https://github.com/MaaAssistantArknights/maa-cli/pull/293)
- Improve roguelike log output and summary by [@wangl-cc](https://github.com/wangl-cc) in [#298](https://github.com/MaaAssistantArknights/maa-cli/pull/298)

### Documentation

- Update docs to sync with MAA main repository by [@wangl-cc](https://github.com/wangl-cc) in [#291](https://github.com/MaaAssistantArknights/maa-cli/pull/291)

### Security

- Bump openssl from 0.10.64 to 0.10.66 by [@dependabot](https://github.com/dependabot) in [#296](https://github.com/MaaAssistantArknights/maa-cli/pull/296)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.8...v0.4.9>

## Release 0.4.8

### Features

- Check git and ssh key availability before hot update by [@wangl-cc](https://github.com/wangl-cc) in [#279](https://github.com/MaaAssistantArknights/maa-cli/pull/279)
- Improve optional value by [@wangl-cc](https://github.com/wangl-cc) in [#280](https://github.com/MaaAssistantArknights/maa-cli/pull/280)
- Subcommand `init` to init profile (asst config) by [@wangl-cc](https://github.com/wangl-cc) in [#282](https://github.com/MaaAssistantArknights/maa-cli/pull/282)
- Support callback message `Destroyed` by [@wangl-cc](https://github.com/wangl-cc)

### Bug Fixes

- Ensure config file are unique after importing by [@wangl-cc](https://github.com/wangl-cc) in [#281](https://github.com/MaaAssistantArknights/maa-cli/pull/281)
- Respect gpu ocr when cpu ocr is not specified by [@wangl-cc](https://github.com/wangl-cc) in [#287](https://github.com/MaaAssistantArknights/maa-cli/pull/287)

### Documentation

- Improve usage guide about running tasks by [@wangl-cc](https://github.com/wangl-cc)
- Fix the link document in the generated docs by [@wangl-cc](https://github.com/wangl-cc)
- Change link target of configuration document by [@wangl-cc](https://github.com/wangl-cc)
- 采用新的文档地址 by [@Alan-Charred](https://github.com/Alan-Charred) in [#289](https://github.com/MaaAssistantArknights/maa-cli/pull/289)
- Fix dead link by [@Cryolitia](https://github.com/Cryolitia)

### Miscellaneous

- Remove debug print by [@wangl-cc](https://github.com/wangl-cc)
- Fix asset links by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.7...v0.4.8>

## Release 0.4.7

### Features

- Mark the old config file `asst.toml` deprecated by [@wangl-cc](https://github.com/wangl-cc) in [#275](https://github.com/MaaAssistantArknights/maa-cli/pull/275)
- New command `import` to import configuration files by [@wangl-cc](https://github.com/wangl-cc) in [#276](https://github.com/MaaAssistantArknights/maa-cli/pull/276)
- Detect device address by `adb devices` by [@wangl-cc](https://github.com/wangl-cc) in [#277](https://github.com/MaaAssistantArknights/maa-cli/pull/277)

### Documentation

- Add docs for other subcommands by [@wangl-cc](https://github.com/wangl-cc)

### Miscellaneous

- Use new file structure for config examples by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.6...v0.4.7>

## Release 0.4.6

### Features

- Support multiple profiles by [@wangl-cc](https://github.com/wangl-cc) in [#251](https://github.com/MaaAssistantArknights/maa-cli/pull/251)
- Add new cleanup targets and some refactoring for `dirs` by [@wangl-cc](https://github.com/wangl-cc) in [#254](https://github.com/MaaAssistantArknights/maa-cli/pull/254)
- Impl `FromStr` for `TaskType` by [@wangl-cc](https://github.com/wangl-cc) in [#262](https://github.com/MaaAssistantArknights/maa-cli/pull/262)
- Read version from environment variable by [@wangl-cc](https://github.com/wangl-cc) in [#273](https://github.com/MaaAssistantArknights/maa-cli/pull/273)
- Handle ResolutionGot and UuidGot messages by [@wangl-cc](https://github.com/wangl-cc) in [#274](https://github.com/MaaAssistantArknights/maa-cli/pull/274)

### Bug Fixes

- Wrong path to item_index.json for non-official clients by [@wangl-cc](https://github.com/wangl-cc) in [#256](https://github.com/MaaAssistantArknights/maa-cli/pull/256)
- Use `$crate::ToCString` in impl_to_cstring macros by [@wangl-cc](https://github.com/wangl-cc)
- Ensure log directory exists before creating log file by [@wangl-cc](https://github.com/wangl-cc) in [#269](https://github.com/MaaAssistantArknights/maa-cli/pull/269)

### Refactor

- Replace lazy_static with OnceLock or normal static by [@wangl-cc](https://github.com/wangl-cc) in [#257](https://github.com/MaaAssistantArknights/maa-cli/pull/257)
- Remove clap_verbosity_flag by [@wangl-cc](https://github.com/wangl-cc) in [#265](https://github.com/MaaAssistantArknights/maa-cli/pull/265)

### Documentation

- Add Nix/Linux by [@Cryolitia](https://github.com/Cryolitia) in [#248](https://github.com/MaaAssistantArknights/maa-cli/pull/248)
- Update README for maa-sys and add more docs by [@wangl-cc](https://github.com/wangl-cc)
- Fix markdown link in doc comments by [@wangl-cc](https://github.com/wangl-cc)
- Split readme and move into `maa-cli/docs` by [@wangl-cc](https://github.com/wangl-cc)
- Fix grammar by [@wangl-cc](https://github.com/wangl-cc) in [#250](https://github.com/MaaAssistantArknights/maa-cli/pull/250)
- Update documentation generation script and titles by [@wangl-cc](https://github.com/wangl-cc)

### Testing

- Compare `BTreeSet` instead of `Vec` in cleanup by [@wangl-cc](https://github.com/wangl-cc) in [#271](https://github.com/MaaAssistantArknights/maa-cli/pull/271)

### Security

- Bump h2 from 0.4.3 to 0.4.4 by [@dependabot](https://github.com/dependabot) in [#252](https://github.com/MaaAssistantArknights/maa-cli/pull/252)

### Miscellaneous

- Use AGPL-3.0-only License by [@wangl-cc](https://github.com/wangl-cc) in [#234](https://github.com/MaaAssistantArknights/maa-cli/pull/234)
- Bump DavidAnson/markdownlint-cli2-action from 15 to 16 by [@dependabot](https://github.com/dependabot) in [#253](https://github.com/MaaAssistantArknights/maa-cli/pull/253)
- Bump Cargo.lock by [@wangl-cc](https://github.com/wangl-cc) in [#258](https://github.com/MaaAssistantArknights/maa-cli/pull/258)
- Remove some unnecessary comments by [@wangl-cc](https://github.com/wangl-cc)
- Fix relative links in generated docs by [@wangl-cc](https://github.com/wangl-cc)
- Use relative md links for generated docs by [@wangl-cc](https://github.com/wangl-cc)
- Update documentation of some methods in cleanup.rs by [@wangl-cc](https://github.com/wangl-cc)
- Update fish completion for cleanup target by [@wangl-cc](https://github.com/wangl-cc)
- Split command definition by [@wangl-cc](https://github.com/wangl-cc) in [#267](https://github.com/MaaAssistantArknights/maa-cli/pull/267)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.5...v0.4.6>

## Release 0.4.5

### Features

- Add env `MAA_LOG_PREFIX` to control prefix in log by [@wangl-cc](https://github.com/wangl-cc) in [#231](https://github.com/MaaAssistantArknights/maa-cli/pull/231)
- Add clap_mangen to generate man page by [@Cryolitia](https://github.com/Cryolitia) in [#236](https://github.com/MaaAssistantArknights/maa-cli/pull/236)
- Add new subcommand `cleanup` by [@hzxjy1](https://github.com/hzxjy1) in [#223](https://github.com/MaaAssistantArknights/maa-cli/pull/223)

### Bug Fixes

- Add `rt-multi-thread` feature for tokio and bump dependencies by [@wangl-cc](https://github.com/wangl-cc) in [#226](https://github.com/MaaAssistantArknights/maa-cli/pull/226)
- Sanity and medicine usage message by [@wangl-cc](https://github.com/wangl-cc) in [#230](https://github.com/MaaAssistantArknights/maa-cli/pull/230)
- Set default log prefix to Always to avoid breaking change by [@wangl-cc](https://github.com/wangl-cc) in [#233](https://github.com/MaaAssistantArknights/maa-cli/pull/233)

### Documentation

- Add link to contributor in changelog by [@wangl-cc](https://github.com/wangl-cc) in [#241](https://github.com/MaaAssistantArknights/maa-cli/pull/241)
- Update default log prefix behavior by [@wangl-cc](https://github.com/wangl-cc) in [#245](https://github.com/MaaAssistantArknights/maa-cli/pull/245)

### Miscellaneous

- Bump version to `0.4.5` and update changelog by [@wangl-cc](https://github.com/wangl-cc) in [#218](https://github.com/MaaAssistantArknights/maa-cli/pull/218)
- Remove unused file `run/fight.rs` by [@wangl-cc](https://github.com/wangl-cc) in [#229](https://github.com/MaaAssistantArknights/maa-cli/pull/229)
- Update fish completion by [@wangl-cc](https://github.com/wangl-cc) in [#232](https://github.com/MaaAssistantArknights/maa-cli/pull/232)
- Add fish completion for cleanup and mangen by [@wangl-cc](https://github.com/wangl-cc) in [#238](https://github.com/MaaAssistantArknights/maa-cli/pull/238)
- Fix lint errors in tests by [@wangl-cc](https://github.com/wangl-cc) in [#243](https://github.com/MaaAssistantArknights/maa-cli/pull/243)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.4...v0.4.5>

## Release 0.4.4

### Features

- Support `timezone` for all time related conditions by [@wangl-cc](https://github.com/wangl-cc) in [#207](https://github.com/MaaAssistantArknights/maa-cli/pull/207)

### Bug Fixes

- Default connection config for linux by [@wangl-cc](https://github.com/wangl-cc) in [#212](https://github.com/MaaAssistantArknights/maa-cli/pull/212)

### Miscellaneous

- Improve changelog generation by [@wangl-cc](https://github.com/wangl-cc) in [#216](https://github.com/MaaAssistantArknights/maa-cli/pull/216)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.3...v0.4.4>

## Release 0.4.3

### Features

- Add `preset` field for connection configuration by [@wangl-cc](https://github.com/wangl-cc) in [#195](https://github.com/MaaAssistantArknights/maa-cli/pull/195)
- Add `client` field to `Weekday` condition used to adjust date by [@wangl-cc](https://github.com/wangl-cc) in [#203](https://github.com/MaaAssistantArknights/maa-cli/pull/203)

### Bug Fixes

- Add newline to summary detail of roguelike by [@wangl-cc](https://github.com/wangl-cc) in [#194](https://github.com/MaaAssistantArknights/maa-cli/pull/194)
- Use 32 bit int and float in `MAAValue` by [@wangl-cc](https://github.com/wangl-cc) in [#198](https://github.com/MaaAssistantArknights/maa-cli/pull/198)

### Documentation

- Fix format of toml example by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.2...v0.4.3>

## Release 0.4.2

### Features

- Add condition `DayMod` for multi-day plan by [@wangl-cc](https://github.com/wangl-cc) in [#190](https://github.com/MaaAssistantArknights/maa-cli/pull/190)

### Bug Fixes

- If start time is later than end, treat it as crossing midnight by [@wangl-cc](https://github.com/wangl-cc) in [#189](https://github.com/MaaAssistantArknights/maa-cli/pull/189)

### Miscellaneous

- Add condition `DayMod` for task schema by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.1...v0.4.2>

## Release 0.4.1

### Performance

- Use `Cow` to avoid unnecessary allocation by [@wangl-cc](https://github.com/wangl-cc) in [#181](https://github.com/MaaAssistantArknights/maa-cli/pull/181)

### Documentation

- Mention that partial installation of MaaCore is not recommended by [@wangl-cc](https://github.com/wangl-cc)

### Miscellaneous

- Fix typos by [@wangl-cc](https://github.com/wangl-cc) in [#179](https://github.com/MaaAssistantArknights/maa-cli/pull/179)
- Rename `as_string` to `as_str` by [@wangl-cc](https://github.com/wangl-cc) in [#182](https://github.com/MaaAssistantArknights/maa-cli/pull/182)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.4.0...v0.4.1>

## Release 0.4.0

### Features

- Search both origin and canonicalized directory of `current_exe` by [@wangl-cc](https://github.com/wangl-cc) in [#94](https://github.com/MaaAssistantArknights/maa-cli/pull/94)
- Add a new subcommand `fight` by [@wangl-cc](https://github.com/wangl-cc) in [#104](https://github.com/MaaAssistantArknights/maa-cli/pull/104)
- Add `BoolInput` to query user for boolean input by [@wangl-cc](https://github.com/wangl-cc) in [#107](https://github.com/MaaAssistantArknights/maa-cli/pull/107)
- Query `start_game_enabled` and `client_type` in startup task by [@wangl-cc](https://github.com/wangl-cc) in [#110](https://github.com/MaaAssistantArknights/maa-cli/pull/110)
- Add subcommand `copilot` to complete the auto-battle feature by [@hzxjy1](https://github.com/hzxjy1) in [#127](https://github.com/MaaAssistantArknights/maa-cli/pull/127)
- **BREAKING**: Resource update and refactor maa core binding by [@wangl-cc](https://github.com/wangl-cc) in [#126](https://github.com/MaaAssistantArknights/maa-cli/pull/126)
- **BREAKING**: Download native binaries instead of universal binaries on macOS by [@wangl-cc](https://github.com/wangl-cc) in [#133](https://github.com/MaaAssistantArknights/maa-cli/pull/133)
- Add stage argument to fight task by [@wangl-cc](https://github.com/wangl-cc) in [#134](https://github.com/MaaAssistantArknights/maa-cli/pull/134)
- Subcommand `roguelike` by [@wangl-cc](https://github.com/wangl-cc) in [#136](https://github.com/MaaAssistantArknights/maa-cli/pull/136)
- Don't run set options test in CI by [@wangl-cc](https://github.com/wangl-cc) in [#143](https://github.com/MaaAssistantArknights/maa-cli/pull/143)
- Auto set remote url based on locale by [@wangl-cc](https://github.com/wangl-cc) in [#141](https://github.com/MaaAssistantArknights/maa-cli/pull/141)
- Add alias for component and update fish completion by [@wangl-cc](https://github.com/wangl-cc) in [#149](https://github.com/MaaAssistantArknights/maa-cli/pull/149)
- **BREAKING**: Launch PlayCover App only on macOS by [@wangl-cc](https://github.com/wangl-cc) in [#152](https://github.com/MaaAssistantArknights/maa-cli/pull/152)
- **BREAKING**: Log with `env_logger` and show task summary when stopped by [@wangl-cc](https://github.com/wangl-cc) in [#153](https://github.com/MaaAssistantArknights/maa-cli/pull/153)
- Add name field to task config, use it in summary by [@wangl-cc](https://github.com/wangl-cc) in [#154](https://github.com/MaaAssistantArknights/maa-cli/pull/154)
- Add `convert` subcommand to convert config file to another format by [@wangl-cc](https://github.com/wangl-cc) in [#156](https://github.com/MaaAssistantArknights/maa-cli/pull/156)
- Read stage activity from StageActivity.json by [@wangl-cc](https://github.com/wangl-cc) in [#159](https://github.com/MaaAssistantArknights/maa-cli/pull/159)
- Add boolean conditions by [@wangl-cc](https://github.com/wangl-cc) in [#161](https://github.com/MaaAssistantArknights/maa-cli/pull/161)
- Better input by [@wangl-cc](https://github.com/wangl-cc) in [#163](https://github.com/MaaAssistantArknights/maa-cli/pull/163)
- Exit with error when taskchain error by [@wangl-cc](https://github.com/wangl-cc) in [#169](https://github.com/MaaAssistantArknights/maa-cli/pull/169)
- **BREAKING**: Return the error when loading SharedLibrary fail by [@wangl-cc](https://github.com/wangl-cc) in [#172](https://github.com/MaaAssistantArknights/maa-cli/pull/172)
- **BREAKING**: Split startup and closedown from fight by [@wangl-cc](https://github.com/wangl-cc) in [#174](https://github.com/MaaAssistantArknights/maa-cli/pull/174)

### Bug Fixes

- Log message by [@wangl-cc](https://github.com/wangl-cc)
- Only open playcover app when using playtools by [@wangl-cc](https://github.com/wangl-cc) in [#137](https://github.com/MaaAssistantArknights/maa-cli/pull/137)
- Dry-run not working by [@wangl-cc](https://github.com/wangl-cc) in [#140](https://github.com/MaaAssistantArknights/maa-cli/pull/140)
- **BREAKING**: Ensure extra share name is a name instead of a path by [@wangl-cc](https://github.com/wangl-cc) in [#160](https://github.com/MaaAssistantArknights/maa-cli/pull/160)

### Refactor

- Use `object!` macro to create `Value::Object` by [@wangl-cc](https://github.com/wangl-cc) in [#105](https://github.com/MaaAssistantArknights/maa-cli/pull/105)
- Rename `TaskList` to `TaskConfig` and add methods by [@wangl-cc](https://github.com/wangl-cc) in [#108](https://github.com/MaaAssistantArknights/maa-cli/pull/108)
- Move common args of `run` in struct `CommonArgs` by [@wangl-cc](https://github.com/wangl-cc) in [#109](https://github.com/MaaAssistantArknights/maa-cli/pull/109)
- Add `Task::new_with_default()` to simplify code by [@wangl-cc](https://github.com/wangl-cc) in [#111](https://github.com/MaaAssistantArknights/maa-cli/pull/111)
- **BREAKING**: Core and cli installer by [@wangl-cc](https://github.com/wangl-cc) in [#118](https://github.com/MaaAssistantArknights/maa-cli/pull/118)
- Rename Value to MAAValue by [@wangl-cc](https://github.com/wangl-cc)
- Detect game ready and close game by TCP connection by [@wangl-cc](https://github.com/wangl-cc) in [#164](https://github.com/MaaAssistantArknights/maa-cli/pull/164)
- Rename `MAATask` to `TaskType` and move to `maa-sys` by [@wangl-cc](https://github.com/wangl-cc) in [#173](https://github.com/MaaAssistantArknights/maa-cli/pull/173)

### Documentation

- Add build options and update usage and config by [@wangl-cc](https://github.com/wangl-cc) in [#132](https://github.com/MaaAssistantArknights/maa-cli/pull/132)
- Correct zh-CN document link by [@hzxjy1](https://github.com/hzxjy1) in [#171](https://github.com/MaaAssistantArknights/maa-cli/pull/171)

### Testing

- Fix test failure on CI caused by create user resource dir by [@wangl-cc](https://github.com/wangl-cc) in [#142](https://github.com/MaaAssistantArknights/maa-cli/pull/142)
- Ignore tests that attempt to create a directory in user space by [@wangl-cc](https://github.com/wangl-cc) in [#144](https://github.com/MaaAssistantArknights/maa-cli/pull/144)

### Miscellaneous

- Fix typos by [@wangl-cc](https://github.com/wangl-cc)
- Remove debug print by [@wangl-cc](https://github.com/wangl-cc)
- Group all non breaking updates into a single PR by [@wangl-cc](https://github.com/wangl-cc) in [#113](https://github.com/MaaAssistantArknights/maa-cli/pull/113)
- Only bump `Cargo.lock` with dependabot by [@wangl-cc](https://github.com/wangl-cc) in [#116](https://github.com/MaaAssistantArknights/maa-cli/pull/116)
- Change copilot input prompt by [@wangl-cc](https://github.com/wangl-cc) in [#135](https://github.com/MaaAssistantArknights/maa-cli/pull/135)
- **BREAKING**: Add JSON schemas and change file structure by [@wangl-cc](https://github.com/wangl-cc) in [#157](https://github.com/MaaAssistantArknights/maa-cli/pull/157)
- Update dependencies by [@wangl-cc](https://github.com/wangl-cc)
- Update `windows-sys` to `windows` by [@wangl-cc](https://github.com/wangl-cc) in [#170](https://github.com/MaaAssistantArknights/maa-cli/pull/170)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.12...v0.4.0>

## Release 0.3.12

### Features

- Load `MaaCore` with name if core dir not found by [@wangl-cc](https://github.com/wangl-cc) in [#70](https://github.com/MaaAssistantArknights/maa-cli/pull/70)
- Add `user_resource` option in asst config by [@wangl-cc](https://github.com/wangl-cc) in [#72](https://github.com/MaaAssistantArknights/maa-cli/pull/72)
- Make log level related options global by [@wangl-cc](https://github.com/wangl-cc) in [#73](https://github.com/MaaAssistantArknights/maa-cli/pull/73)
- Add `--dry-run` option to `run` command by [@wangl-cc](https://github.com/wangl-cc) in [#76](https://github.com/MaaAssistantArknights/maa-cli/pull/76)
- Support Windows by [@wangl-cc](https://github.com/wangl-cc) in [#77](https://github.com/MaaAssistantArknights/maa-cli/pull/77)
- Better error message when directory not found by [@wangl-cc](https://github.com/wangl-cc)
- Add support for static options by [@wangl-cc](https://github.com/wangl-cc) in [#88](https://github.com/MaaAssistantArknights/maa-cli/pull/88)

### Bug Fixes

- Canonicalize returned path of `current_exe` by [@wangl-cc](https://github.com/wangl-cc) in [#71](https://github.com/MaaAssistantArknights/maa-cli/pull/71)
- `user_resource` should be a flag instead of an option by [@wangl-cc](https://github.com/wangl-cc) in [#74](https://github.com/MaaAssistantArknights/maa-cli/pull/74)
- Load client resource when playtools is not true by [@wangl-cc](https://github.com/wangl-cc) in [#75](https://github.com/MaaAssistantArknights/maa-cli/pull/75)
- Failed to exit on windows by [@wangl-cc](https://github.com/wangl-cc) in [#79](https://github.com/MaaAssistantArknights/maa-cli/pull/79)
- `current_exe` on windows and all platform without `self` feature by [@wangl-cc](https://github.com/wangl-cc) in [#78](https://github.com/MaaAssistantArknights/maa-cli/pull/78)

### Documentation

- Remove outdated comment by [@wangl-cc](https://github.com/wangl-cc)
- Update README to match the latest version by [@wangl-cc](https://github.com/wangl-cc)

### Testing

- Add tests for `ClientType` and fix typo by [@wangl-cc](https://github.com/wangl-cc) in [#85](https://github.com/MaaAssistantArknights/maa-cli/pull/85)

### Miscellaneous

- Add cliff.toml to generate changelog when release by [@wangl-cc](https://github.com/wangl-cc)
- Add some metadata to Cargo.toml by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.11...v0.3.12>

## Release 0.3.11

### Bug Fixes

- Make `Array` higher priority than `Input*` in `Value` by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.10...v0.3.11>

## Release 0.3.10

### Features

- Allow user input in task definition by [@wangl-cc](https://github.com/wangl-cc) in [#54](https://github.com/MaaAssistantArknights/maa-cli/pull/54)
- Add option `strategy` for variant by [@wangl-cc](https://github.com/wangl-cc) in [#64](https://github.com/MaaAssistantArknights/maa-cli/pull/64)

### Documentation

- Change "MaaTouch" to "MAATouch" by [@hzxjy1](https://github.com/hzxjy1) in [#53](https://github.com/MaaAssistantArknights/maa-cli/pull/53)

### Testing

- Add test for deserializing value with input by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.9...v0.3.10>

## Release 0.3.9

### Features

- `MAA_EXTRA_SHARE_NAME` to specify extra share name at compile time by [@wangl-cc](https://github.com/wangl-cc) in [#43](https://github.com/MaaAssistantArknights/maa-cli/pull/43)
- Add feature `self` to disable self update by disable it by [@wangl-cc](https://github.com/wangl-cc) in [#44](https://github.com/MaaAssistantArknights/maa-cli/pull/44)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.8...v0.3.9>

## Release 0.3.8

### Bug Fixes

- Don't clear resource dir if `no_resource` is true by [@wangl-cc](https://github.com/wangl-cc) in [#41](https://github.com/MaaAssistantArknights/maa-cli/pull/41)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.7...v0.3.8>

## Release 0.3.7

### Features

- Add `core` field to CLI configuration by [@wangl-cc](https://github.com/wangl-cc) in [#38](https://github.com/MaaAssistantArknights/maa-cli/pull/38)
- Add completion by [@wangl-cc](https://github.com/wangl-cc) in [#39](https://github.com/MaaAssistantArknights/maa-cli/pull/39)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.6...v0.3.7>

## Release 0.3.6

### Features

- Import download by [@wangl-cc](https://github.com/wangl-cc) in [#36](https://github.com/MaaAssistantArknights/maa-cli/pull/36)

### Bug Fixes

- Handle symlink when extract zip file by [@wangl-cc](https://github.com/wangl-cc) in [#37](https://github.com/MaaAssistantArknights/maa-cli/pull/37)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.5...v0.3.6>

## Release 0.3.5

### Features

- Load resources based on `client_type` and if use `PlayTools` by [@wangl-cc](https://github.com/wangl-cc) in [#33](https://github.com/MaaAssistantArknights/maa-cli/pull/33)

### Bug Fixes

- Don't skip file with same size by [@wangl-cc](https://github.com/wangl-cc) in [#34](https://github.com/MaaAssistantArknights/maa-cli/pull/34)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.4...v0.3.5>

## Release 0.3.4

### Features

- Add CLI config file by [@wangl-cc](https://github.com/wangl-cc) in [#31](https://github.com/MaaAssistantArknights/maa-cli/pull/31)

### Testing

- Fix config path on linux by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.3...v0.3.4>

## Release 0.3.3

### Features

- Drop assistant on Ctrl+C by [@horror-proton](https://github.com/horror-proton) in [#29](https://github.com/MaaAssistantArknights/maa-cli/pull/29)

### Bug Fixes

- Don't ensure `lib_dir` clean by [@wangl-cc](https://github.com/wangl-cc) in [#30](https://github.com/MaaAssistantArknights/maa-cli/pull/30)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.2...v0.3.3>

## Release 0.3.2

### Bug Fixes

- Version parsing of MaaCore by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.1...v0.3.2>

## Release 0.3.1

### Features

- Cross compile for arm64 linux by [@wangl-cc](https://github.com/wangl-cc) in [#25](https://github.com/MaaAssistantArknights/maa-cli/pull/25)

### Documentation

- Update README by [@wangl-cc](https://github.com/wangl-cc)
- 中文作为主README，英文作为README-EN.md by [@wangl-cc](https://github.com/wangl-cc)
- Update badge by [@wangl-cc](https://github.com/wangl-cc)

**Full Changelog**: <https://github.com/MaaAssistantArknights/maa-cli/compare/v0.3.0...v0.3.1>

## Release 0.3.0

### Features

- Better maa callback based on MacGUI by [@wangl-cc](https://github.com/wangl-cc)
- Print lib and resource dir by [@wangl-cc](https://github.com/wangl-cc)
- **BREAKING**: Rename touch modes and default to ADB by [@wangl-cc](https://github.com/wangl-cc)
- **BREAKING**: Support PlayCover connection by [@wangl-cc](https://github.com/wangl-cc)
- Install package with `maa-updater` by [@wangl-cc](https://github.com/wangl-cc)
- Speed test from mirrors and fix for linux by [@wangl-cc](https://github.com/wangl-cc)
- Improve speed test of updater by [@wangl-cc](https://github.com/wangl-cc)
- Make all filename parameters relative to config directory by [@wangl-cc](https://github.com/wangl-cc)
- Improve start and close game of playcover mode by [@wangl-cc](https://github.com/wangl-cc)
- Extract for windows by [@wangl-cc](https://github.com/wangl-cc)
- **BREAKING**: Maa-updater can only install from prebuilt package by [@wangl-cc](https://github.com/wangl-cc)
- Download from GitHub release in CI by [@wangl-cc](https://github.com/wangl-cc)
- **BREAKING**: Remove mod type by [@wangl-cc](https://github.com/wangl-cc)
- Improve build script by [@wangl-cc](https://github.com/wangl-cc)
- **BREAKING**: Drop support for windows and fix test on macOS by [@wangl-cc](https://github.com/wangl-cc) in [#5](https://github.com/MaaAssistantArknights/maa-cli/pull/5)
- **BREAKING**: Maa-run as a subcommand of maa-helper to set env vars for maa-run by [@wangl-cc](https://github.com/wangl-cc) in [#7](https://github.com/MaaAssistantArknights/maa-cli/pull/7)
- **BREAKING**: Import help and version will only show MaaCore version by [@wangl-cc](https://github.com/wangl-cc) in [#12](https://github.com/MaaAssistantArknights/maa-cli/pull/12)
- **BREAKING**: More useful `maa` command by [@wangl-cc](https://github.com/wangl-cc) in [#13](https://github.com/MaaAssistantArknights/maa-cli/pull/13)
- Add yaml support for config file by [@wangl-cc](https://github.com/wangl-cc) in [#14](https://github.com/MaaAssistantArknights/maa-cli/pull/14)
- Add a field `resources` to specify additional resource files by [@wangl-cc](https://github.com/wangl-cc) in [#15](https://github.com/MaaAssistantArknights/maa-cli/pull/15)
- Impl ToCString for both PathBuf and &PathBuf by [@wangl-cc](https://github.com/wangl-cc)
- Better log system and message handling by [@wangl-cc](https://github.com/wangl-cc) in [#17](https://github.com/MaaAssistantArknights/maa-cli/pull/17)
- Failed message will be printed to debug log by [@wangl-cc](https://github.com/wangl-cc) in [#18](https://github.com/MaaAssistantArknights/maa-cli/pull/18)
- Support absolute path for additional resource by [@wangl-cc](https://github.com/wangl-cc) in [#19](https://github.com/MaaAssistantArknights/maa-cli/pull/19)
- New option `--user-resource` by [@wangl-cc](https://github.com/wangl-cc)

### Bug Fixes

- Typo by [@wangl-cc](https://github.com/wangl-cc)
- Extract on linux by [@wangl-cc](https://github.com/wangl-cc)
- Extract on linux by [@wangl-cc](https://github.com/wangl-cc)
- Regex to match python files by [@wangl-cc](https://github.com/wangl-cc)
- Don't use default features of chrono by [@wangl-cc](https://github.com/wangl-cc)
- Check if file is dir instead of outpath by [@wangl-cc](https://github.com/wangl-cc)
- Asset_name for windows by [@wangl-cc](https://github.com/wangl-cc)
- Message handling by [@wangl-cc](https://github.com/wangl-cc) in [#6](https://github.com/MaaAssistantArknights/maa-cli/pull/6)
- Wrong match in get_asset by [@wangl-cc](https://github.com/wangl-cc)
- Name of `maa-cli` should be `maa` by [@wangl-cc](https://github.com/wangl-cc)
- Download url with `MAA_CLI_DOWNLOAD` by [@wangl-cc](https://github.com/wangl-cc)
- Remove duplicate log for additional resource by [@wangl-cc](https://github.com/wangl-cc)
- Don't treat other error as file not found during parse asst_config by [@wangl-cc](https://github.com/wangl-cc)
- Yaml support by [@wangl-cc](https://github.com/wangl-cc)

### Refactor

- Better error handle by [@wangl-cc](https://github.com/wangl-cc)
- **BREAKING**: Remove maa-util, split maa-sys and other improves by [@wangl-cc](https://github.com/wangl-cc)
- **BREAKING**: Rename `maa-runner` to `maa-cli` by [@wangl-cc](https://github.com/wangl-cc)
- **BREAKING**: Rename workspace members to avoid confusion by [@wangl-cc](https://github.com/wangl-cc) in [#9](https://github.com/MaaAssistantArknights/maa-cli/pull/9)
- **BREAKING**: Remove maa-run and use dlopen to load MaaCore by [@wangl-cc](https://github.com/wangl-cc) in [#24](https://github.com/MaaAssistantArknights/maa-cli/pull/24)

### Documentation

- Add installation and usage by [@wangl-cc](https://github.com/wangl-cc)
- Add README-CN by [@wangl-cc](https://github.com/wangl-cc)
- Add feature and update todo by [@wangl-cc](https://github.com/wangl-cc)
- Add CHANGELOG of maa-cli by [@wangl-cc](https://github.com/wangl-cc)
- Clean CHANGELOG by [@wangl-cc](https://github.com/wangl-cc)
- Update README by [@wangl-cc](https://github.com/wangl-cc)
- Add doc about log level by [@wangl-cc](https://github.com/wangl-cc)
- Add note for `adb` by [@wangl-cc](https://github.com/wangl-cc)
- Add notice for `adb` by [@wangl-cc](https://github.com/wangl-cc)
- Doc about create config dir by [@wangl-cc](https://github.com/wangl-cc)
- Explain which binary to download for macOS and Linux by [@wangl-cc](https://github.com/wangl-cc)

### Testing

- Add AsstConfig test and fix get_dir test by [@wangl-cc](https://github.com/wangl-cc)

### Miscellaneous

- Lint by clippy and fmt by [@wangl-cc](https://github.com/wangl-cc)
- Apply clippy by [@wangl-cc](https://github.com/wangl-cc)
- Move example to config_examples to avoid confusion by [@wangl-cc](https://github.com/wangl-cc)
- Change package name to maa-cli by [@wangl-cc](https://github.com/wangl-cc)
- Update comment by [@wangl-cc](https://github.com/wangl-cc)

<!-- markdownlint-configure-file
{
  "line-length": false,
  "no-duplicate-heading": {
    "siblings_only": true
  }
}
-->

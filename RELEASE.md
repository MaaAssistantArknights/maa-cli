# maa-cli 发布流程

本文档面向维护者，说明 maa-cli 的分支、版本和发布方式。日常开发参见 [开发文档](DEVELOPMENT.md)。

## 原则

- 版本只来自 `crates/maa-cli/Cargo.toml`：每个分支的 Cargo 版本就是这个分支的版本。
- Stable 只从 `main` 发布，由带 `release` 标签的同仓库 PR 合入触发。Cargo 版本可以由任何 PR 修改；修改 `Cargo.toml` 本身不会触发发布。
- Beta 和 Nightly 只从 `dev-X.Y` 分支发布。没有 dev 分支时既没有 Beta 也没有 Nightly。
- 发布工具（workflow 和 `cargo x -- release`）总是使用 `main` 上的版本，dev 分支不需要同步 CI 配置也能正确发布。

## 分支规范

| 分支 | 作用 | Cargo 版本 |
| --- | --- | --- |
| `main` | 只发 Stable，保持随时可发布 | 当前或即将发布的 Stable 版本 |
| `dev-X.Y` | 大型或破坏性更新（如 0.8），发 Beta 和 Nightly | 创建时设为 `X.Y.0`，且高于最新 Stable |

- 同一时间最多一个 dev 分支；存在多个 `dev-*` 时 Beta 和 Nightly 都会失败。分支名与 Cargo 版本不一致，或 `X.Y.0` 已不高于最新 Stable 时，Beta 失败、Nightly 跳过。
- 定期把 `main` 合入 dev 分支，不要反向合并。`main` 每次发布 Stable 后，合并时 `Cargo.toml` 和 `Cargo.lock` 的版本行会冲突，保留 dev 分支的 `X.Y.0`。CI 同样在 `dev-*` 的 PR 和 merge queue 上运行。
- 不要改写 dev 分支的历史（rebase、force-push）。Nightly 编号来自提交数，改写后编号可能不高于已发布的 Nightly，此时 Nightly 会失败而不是发布更低的版本，直到提交数超过已发布的编号。
- dev 分支完成后，它合回 `main` 的 PR 本身就是 release PR（见下文），必须用 merge commit 合并，不要 squash 或 rebase，以保留每个提交和 Beta tag 的历史。merge queue 若只允许 squash，需临时改为 merge commit。发布后删除 dev 分支，并把仍指向它的 PR 改为指向 `main`。

## 通道

| 通道 | 怎么发 | 版本 | tag |
| --- | --- | --- | --- |
| Stable | release PR 合入 `main` | Cargo 版本 `X.Y.Z` | `vX.Y.Z` |
| Beta | 在 `main` 上手动运行 `Release`，构建 dev 分支 | `X.Y.0-beta.N` | `vX.Y.0-beta.N` |
| Nightly | 每日定时，dev 分支有新提交时发布 | `X.Y.0-nightly.C+sha.<short>` | `nightly`（可移动） |

- Beta 编号 `N` 取已有 `vX.Y.0-beta.*` tag 的最大值加一；dev 分支 HEAD 已有 Beta tag 时跳过。
- Nightly 的 `C` 是 dev 分支的提交数（`git rev-list --count HEAD`），随历史单调递增，同一提交重跑得到同一版本；`nightly` tag 已指向 HEAD 时跳过。
- 同一核心版本内的顺序为 `X.Y.0-beta.N` < `X.Y.0-nightly.C` < `X.Y.0`。
- 预发布通过 `MAA_VERSION` 注入编译，不修改源码中的版本和 changelog。预发布说明只包含最近一个可达 Stable tag 之后的提交，不会重复列出已发布的 Stable 修复。

## 版本索引

`version` 分支保存 `stable.json`、`beta.json`、`nightly.json` 及对应 `.txt`；`alpha.json` 与 `nightly.json` 内容相同：客户端沿用 MaaCore 的通道名 alpha，安装脚本也读取它。发布计划只读 `nightly.json`，它不存在时视为空通道，由第一次 Nightly 创建。

通道从稳定到不稳定依次为 Stable → Beta → Nightly。一次发布会更新它所在的通道，以及更不稳定、且当前版本低于它的通道：例如 dev 分支已有 `0.8.0-beta.1` 时，发布 `0.7.6` 只更新 `stable.json`；发布 `0.8.0` 则三个通道都更新。要更新的通道在发布计划中一次算定；更新索引前若这些通道已被其他发布改动，任务失败而不覆盖。

下游按同样的结果发布：更新 Stable 时发布 Homebrew `maa-cli`、AUR 和 WinGet，并移动 `stable` tag；更新 Beta 时发布 Homebrew `maa-cli-beta`。新版本发布后不要重跑旧版本的下游任务：Homebrew 会拒绝降级，WinGet 会新增旧版本，AUR 会被回退。

## 发布 Stable

Stable 只从 `main` 发布：

1. 在 Actions 中从 `main` 运行 `Prepare Stable Release`，选择 `version`：
   - `auto`（默认）：Cargo 版本高于最近正式 tag 时沿用，否则由 git-cliff 推导。`0.x` 阶段 `fix` 和 `feat` 都推进 patch，breaking change 推进 minor；只有文档、重构等变更时不会自动 bump。
   - `patch`、`minor`、`major`：由 git-cliff 相对最近正式 tag 计算。
   - `X.Y.Z`：直接指定，必须高于最近正式 tag。
2. 工作流更新 Cargo 版本和 lockfile，用 git-cliff 生成 changelog，创建或更新带 `release` 标签的 `release-prep/vX.Y.Z` PR，并关闭其他打开的 `release-prep/*` PR。版本已由其他 PR 更新时，本次只修改 changelog。
3. 批准自动创建 PR 的 workflow runs，检查 CI 后合并。

dev 分支先通过普通 PR 合回 `main`，必须使用 merge commit，不能 squash 或 rebase，这样 Beta tag 仍在历史中。合并后 `main` 的 Cargo 版本为 `X.Y.0`，再按上面的步骤运行 Prepare，`auto` 会沿用 `X.Y.0`。

合并后、创建任何 tag 前，`Release` 会按实际合并提交重新生成 changelog 并与 `CHANGELOG.md` 比对。若准备之后 `main` 又合入了会进入 changelog 的提交，发布会在这里失败，不会发出错误版本；此时 Cargo 版本已在 `main` 上，再运行一次 Prepare 即可创建只修复 changelog 的 release PR。

不符合约定式提交格式的 commit（如网页编辑生成的 `Update README.md`）不会阻止版本推导或发布，只是不出现在 changelog 中；GitHub Revert 按钮生成的 `Revert "..."` 会被归入 Revert 分组。

## 发布 Beta 和 Nightly

Beta：在 Actions 中从 `main` 运行 `Release`，选择 `channel=beta`，真正发布时勾选 `publish`；不勾选只构建预览并在运行摘要中显示发布说明。Nightly 每日自动运行，也可以选择 `channel=nightly` 手动运行。

## 重试与恢复

同一次 workflow run 的发布计划和说明保存为 `release-plan` artifact，打包结果和索引清单保存为 `release-bundle` artifact。重跑全部或失败的 jobs 都复用已保存的内容，不重新分配版本或改写产物。版本索引更新失败时，重跑 `Update Version Index` 及其后续任务即可，不需要重新编译。

编译产物只保留 7 天：若 `Publish Release` 在生成 `release-bundle` 前失败且已超过 7 天，需要选择「Re-run all jobs」。plan 和 bundle 保留 90 天；过期后工作流停止，应重新发起发布。正式 tag 已存在时必须指向同一提交。

## 仓库设置

`Prepare Stable Release` 使用内置 `GITHUB_TOKEN`，以 `github-actions[bot]` 身份推送分支和创建 PR。需要在 Settings → Actions → General 中启用 `Allow GitHub Actions to create and approve pull requests`。具有 write 权限的维护者批准 PR 的 workflow runs 后检查才会运行。合并由维护者完成，不使用 `GITHUB_TOKEN` 自动合并，否则不会产生后续发布事件。

修改发布工具后运行 `cargo test -p xtask` 和 `cargo clippy -p xtask --all-targets -- -D warnings`。部分测试会在临时仓库中运行 git 和 Cargo，需要在 PATH 中提供 git-cliff 2.13.1。

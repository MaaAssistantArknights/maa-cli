# maa-cli 发布流程

本文档面向维护者，说明版本准备、发布和失败恢复。日常开发参见 [开发文档](DEVELOPMENT.md)。

## 发布入口

| 通道 | 触发方式 | 编译版本 | Git tag |
| --- | --- | --- | --- |
| Stable | 推送 `vX.Y.Z` tag | `X.Y.Z` | `vX.Y.Z` |
| Beta | 推送 `vX.Y.Z-beta.N` tag | `X.Y.Z-beta.N` | `vX.Y.Z-beta.N` |
| Nightly | 每日 20:00 UTC 定时或在 `main` 手动运行 `Release` | `X.Y.0-nightly.T+sha.<short>` | `nightly`（滚动更新） |

Stable 和 Beta 的完整版本由 tag 决定，tag 的核心版本必须与对应提交的 Cargo 版本一致。完整版本通过 `MAA_VERSION` 注入编译，不从源码推导版本或自动递增 Beta 编号。修改版本、合并 PR 本身不触发发布；其他预发布后缀不接受，删除 tag 不触发发布。版本 tag 不应移动。

Nightly 选择唯一的 `dev-*` 分支，读取其源码版本并添加后缀。没有 dev 分支时跳过，存在多个时失败。`T` 是 Meta job 准备发布时取一次的 Unix 时间戳（秒）；SHA 标识源码提交，编号不依赖提交数。同一核心版本按 SemVer 排序为 `beta.N` < `nightly.T` < Stable。已有正式发布的 Nightly 指向同一源码提交时跳过；仅存在草稿或 tag 不会跳过。

手动运行默认只构建预览，勾选 `publish` 才发布。面向 `main` 的 PR 只有修改 `.github/workflows/release.yml` 才触发预览，使用 PR 的合并测试提交，不查找 dev 分支。预览使用 `dev` profile，正式发布使用 `release-lto`；两者均执行构建、打包和发布说明生成，预览不创建 Release、更新索引或触发下游。

## 分支与源码

`main` 保持可发布 Stable；长期开发大型或破坏性更新时使用 `dev-X.Y`，创建时将源码版本设为高于最新 Stable 的 `X.Y.0`。短分支合入 `main` 无需先发布 Beta。CI 和文档检查覆盖 `dev-*` 的 push、PR 和 merge queue。

将 `main` 中适用的修复通过 cherry-pick 同步到 dev，保留 dev 的版本号。首次发布 Beta 前可以按协作需要 rebase；发布 Beta 后，不再 merge 或 rebase `main`，也不要改写已有 Beta 的历史，确保前一 Beta 仍是后续发布的祖先。Beta 说明以当前提交可达的版本 tag 划定范围，直接合入 main 的 Stable tag 会改变这个边界。

dev 完成后，在合回 `main` 的 PR 中准备版本和 changelog；也可以合并后另开 PR 补齐，再打 Stable tag。合并时使用 merge commit 保留 dev 历史，不要 squash 或 rebase；仓库或 merge queue 若限制为线性历史，需要先调整合并方式。发布后删除 dev 分支，并调整仍指向它的 PR。

Stable/Beta 使用 tag 对应提交中的 workflow、源码和工具。Nightly 的 workflow 与打包工具来自启动运行的 `main` 提交，源码和 `cargo x build` 来自选定的 dev 提交。因此 dev 开始发布前也必须同步当前构建工具。Meta 输出统一的源码 `ref`（提交 SHA），构建、发布说明和 manifest 均使用它；Meta 不拉取完整历史。

## 准备和发布 Stable

发布前准备好 Cargo 版本，以及 `CHANGELOG.md` 中的 `## Release X.Y.Z` 段落。可以直接在 PR 中准备，也可以运行 `Prepare Stable Release`：

1. Prepare 固定 checkout `main`，由 git-cliff 对未发布提交自动建议版本。`0.x` 阶段 `feat` 和 `fix` 推进 patch，breaking change 推进 minor；没有未发布的 feat、fix 或 breaking change 时失败。
2. 工作流更新 `Cargo.toml`、`Cargo.lock` 和 changelog，创建或更新固定 `release-prep` 分支的 PR。需要其他版本时，在这个 PR 中同步修改三者即可。
3. 审阅版本和说明，批准自动创建 PR 的 workflow runs，检查 CI 后合并。
4. 在最终准备发布的 `main` 提交上创建并推送 `vX.Y.Z` tag。

Prepare 不发布任何版本。重新运行会从当次 main 重建准备分支，覆盖尚未合入 main 的手工修改；需要保留的修改应重新应用。Prepare 合并后应完成打 tag，再准备下一版。

Stable 的 GitHub Release 说明直接读取 tag 对应提交的 changelog 段落；缺少该段落时发布失败。准备之后合入的其他提交不会自动出现在说明中，需要在发布前补入。读取 Stable 说明只需当前提交，无需完整 Git 历史。

Prepare 使用内置 `GITHUB_TOKEN`。仓库需要启用 Settings → Actions → General → `Allow GitHub Actions to create and approve pull requests`；自动创建 PR 的 workflow runs 由具有 write 权限的维护者批准。合并和打 tag 由维护者完成。

## Beta 和 Nightly 说明

Beta 在待发布提交上推送 `vX.Y.Z-beta.N` tag，不要求提前准备 changelog。git-cliff 从前一个可达的版本 tag 生成增量说明：通常是同系列前一个 Beta；首次 Beta 通常从分支起点的 Stable 开始。若在同一 dev 历史中直接切换核心版本，之前系列的 Beta 也会成为边界。期间 cherry-pick 的修复会列入说明。

Nightly 使用最近可达 Stable tag 之后的提交生成说明。预发布说明由独立 job 获取完整历史生成，与构建并行执行；源码版本和 changelog 不会被改写。

## 索引与下游

`version` 分支保存 `stable.json`、`beta.json`、`nightly.json` 及对应 `.txt`。更新 Nightly 时同时写入相同内容的 `alpha.json` 和 `alpha.txt`，兼容旧客户端与安装脚本。

Nightly 每次发布都更新 `nightly` 和 `alpha`，不比较版本，保证索引始终指向当前的滚动 Release。Beta 只更新 `beta`；Stable 分别尝试更新 Stable、Beta、Nightly。Stable 和 Beta 按 SemVer 比较 manifest 与索引的 `version` 字段，忽略 build metadata：较新版本更新，较旧版本跳过，相同版本允许失败任务重试，缺失索引可以创建。渠道选择不读取 dev 分支，也不比较 manifest 的其他字段。

例如发布 `0.7.6` 时，已经指向 `0.8.0` 预发布的 Beta/Nightly 不会被覆盖，但 Stable 仍可更新；发布 `0.8.0` 时则会一并推进较旧的预发布通道。发布较旧 tag 仍可创建独立 Release，但不会回退索引。

Stable Release 发布成功后，独立更新版本索引、移动 `stable` tag，并触发 AUR 和 WinGet。索引更新不是 tag 或第三方发布的前置条件。Stable 或 Beta 发布后，Homebrew 通过各 formula 的 livecheck 自行判断更新，无需传入版本或渠道。WinGet 从正式 tag 单独构建自己的包，追加到对应 Release；重试复用已有附件。

Nightly 只保留滚动 Release 的最新版，不为每次构建创建永久 tag。不要手动重跑已被新版本取代的旧 Nightly 发布，以免覆盖当前滚动 Release。发布运行串行排队，PR 预览使用独立并发队列。

## 构建与恢复

`cargo x build --output-dir <目录>` 为发布准备二进制和按目标平台、features 生成的 `licenses.md`，需要安装 `cargo-about`。workflow 负责跨 job 的 tar，WinGet 直接使用输出目录。`cargo x --release -- release package` 使用 `VERSION`、`TAG`、`COMMIT` 和构建 artifact 生成发行包、`manifest.json`、`manifest.txt`；不修改版本索引。

`cargo x -- release channels` 从 `CHANNEL`、`MANIFEST`（JSON 文件路径）、`INDEX_DIR` 读取输入，将可更新通道写入 `GITHUB_OUTPUT` 的 `channels`，用于发布后的索引更新。

普通发行包上传到草稿后才正式发布；已发布的版本 Release 不会被覆盖。失败后使用「Re-run failed jobs」，复用成功 job 的版本号和构建产物；重新运行 Meta 会为 Nightly 生成新的时间戳。不要对已经发布的版本使用「Re-run all jobs」，重新编译可能改变 checksum。构建 artifact 和 manifest 保留 7 天。

较新 Stable 发布后，不要再发布旧版 Stable 或重跑旧版的 AUR、`stable` tag 更新任务，以免使 AUR 或 `stable` tag 回退。手动运行 AUR workflow 也应只用于恢复当前版本。

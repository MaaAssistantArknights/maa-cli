# maa-cli 开发文档

## 许可证

本项目采用 AGPL-3.0-only 许可证。您的所有贡献都将被纳入本项目，并遵循相同的许可证。

## 贡献指南

### 贡献流程

1. **Fork 本仓库**：在 GitHub 上 fork 本项目到你的个人账户。

2. **创建分支**：从主分支（`main` 或 `master`）拉取最新代码，并基于此创建分支（如 `feature/xxx`、`fix/xxx`）。

3. **开发与提交**：按照[代码规范](#代码规范)进行开发，确保代码格式、质量和测试覆盖率达标。建议每次提交前运行 `cargo +nightly fmt` 和 `cargo clippy`。

4. **推送分支并发起 PR**：将你的分支推送到 fork 仓库，并发起 Pull Request（PR）。PR 标题和描述需遵循 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/) 规范，简明扼要说明变更内容和动机，并在 PR 中关联相关 Issue。

5. **代码评审与修改**：项目维护者会尽快进行代码评审，并提出修改建议。

6. **合并与发布**：通过评审后，PR 会以 squash 方式合并，你的贡献将被记录在 Change Log 中。

### 注意事项

- 避免直接向主分支提交代码，始终通过 PR 进行贡献。
- 提交 PR 前，确保自己的分支已经与主分支同步，避免合并冲突。
- 对于较大或影响范围广的变更，建议先在 Issue 中充分讨论方案。
- 如对现有代码有任何疑问，可以在 Issue 中提出，以获得帮助和反馈。
- 欢迎任何形式的贡献，包括文档、测试、CI 配置等。

## 开发环境

### 环境要求

- **Rust 工具链**：需要 1.84 版本或更高。推荐使用 [rustup](https://rustup.rs/) 安装。
  - 安装 nightly 版本的 `rustfmt`：`rustup component add rustfmt --toolchain nightly`
- **C 编译器**：如需在 Linux 上从源码编译 OpenSSL，或启用 `git2` 功能（默认启用）时需要。建议关闭 `git2` 功能以避免依赖 C 编译器和额外的 OpenSSL 库。待 gix 完全替代 git2 后，将移除该依赖。

### 构建项目

```bash
# 调试构建
cargo build

# 发布构建
cargo build --release

# 构建特定 crate
cargo build -p maa-cli
```

注意：不要使用 `--all-features`，否则 `cargo` 会强制从源码编译 OpenSSL，严重拖慢构建速度。

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定 crate 的测试
cargo test -p maa-cli

# 运行特定测试
cargo test <测试名称>
```

## 代码规范

- **格式化**：使用 nightly 版 `rustfmt` 格式化代码。提交前请运行 `cargo +nightly fmt` 保证格式一致。
- **质量检查**：使用 `cargo clippy` 检查代码质量。任何警告都会导致 CI 失败。不可避免的警告请用 `#[allow]` 或 `#[expect]` 注明原因。
- **Unsafe**：尽量避免 `unsafe`。如必须使用，请添加注释解释原因。
- **错误处理**：
  - `maa-cli` 使用 `anyhow`。
  - 其他组件使用 `thiserror` 或者自行编写错误类型来处理错误。
  - 避免 `unwrap` 和 `expect`，如必须使用请注释说明原因。
- **测试规范**：
  - 新代码应尽量编写测试，修复旧代码时请添加相关 bug 测试。
  - MaaCore FFI 功能可省略测试，但需本地实际运行验证。
  - 需网络或读写系统/用户文件的测试请用 `#[ignore]` 标记，避免在本地和沙盒环境运行。可用 `cargo test -- --ignored` 运行这些测试。
  - 项目用 `cargo llvm-cov` 生成测试覆盖率报告，并通过 `codecov` 跟踪。PR 会自动生成覆盖率报告，不建议本地运行。
- **依赖管理**：如需新增依赖，请优先考虑社区活跃、维护良好的库，并在 PR 说明中注明用途。

## 文档规范

- 命令及配置的新增或修改需同步更新文档。
- 文档以简体中文为主，英文为辅，其他语言（繁体、韩文、日文）尽量翻译，无法翻译时可用简体中文占位。
- 所有文档为 Markdown 格式，使用 [markdownlint-cli2](https://github.com/DavidAnson/markdownlint-cli2) 检查。
- 段落内不换行，每段仅一行，段落间空行分隔。非段落换行请用 `<br>`，不要用尾随空格。

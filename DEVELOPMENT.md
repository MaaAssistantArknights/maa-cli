# maa-cli 开发文档

## 许可证

本项目采用 AGPL-3.0-only 许可证。您的所有贡献都将被纳入本项目，并遵循相同的许可证。

## 项目结构

本项目采用工作空间（Workspace）结构组织代码。目前包含以下子项目：

- **maa-cli**：主应用程序，提供命令行接口和具体功能实现
- **maa-dirs**：处理 MAA 相关目录管理
- **maa-sys**：提供与 MaaCore 底层库的绑定和安全封装
- **maa-types**：定义核心数据类型

项目结构如下：

```
maa-cli/
├── .cargo/            # Cargo 配置
├── .github/           # GitHub 工作流和配置
├── crates/            # 项目的各个 crate 组件
│   ├── maa-cli/       # 主命令行应用
│   │   ├── completions/      # 命令补全脚本，后续可能删除，由 clap 自动生成
│   │   ├── config_examples/  # 配置文件示例，同时用于测试
│   │   ├── docs/             # 多语言文档
│   │   ├── schemas/          # JSON Schema 文件，后续可能删除，由 schemars 自动生成
│   │   └── src/              # 源代码
│   ├── maa-dirs/      # 目录管理 crate
│   ├── maa-sys/       # 系统接口 crate，提供底层绑定
│   └── maa-types/     # 类型定义 crate
├── Cargo.toml         # 工作空间配置
├── Cargo.lock         # 依赖锁定文件
└── ...                # 其他配置文件
```

## 开发环境

- **Rust 工具链**：需要 1.84 版本或更高。推荐使用 [rustup](https://rustup.rs/) 安装。
- **C 编译器**：如需在 Linux 上从源码编译 OpenSSL，或启用 `git2` 功能（默认启用）时需要。建议关闭 `git2` 功能以避免依赖 C 编译器和额外的 OpenSSL 库。待 gix 完全替代 git2 后，将移除该依赖。

## 构建与测试

### 构建项目

```bash
# 调试构建
cargo build

# 发布构建
cargo build --release

# 构建特定 crate
cargo build -p maa-cli
```

注意：不要使用 `--all-features`，否则会强制从源码编译 OpenSSL，严重拖慢构建速度。

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

- **格式化**：使用 nightly 版 `rustfmt` 格式化代码。提交前请运行 `cargo +nightly fmt` 保证格式一致。可通过 `rustup component add rustfmt --toolchain nightly` 安装。
- **质量检查**：使用 `cargo clippy` 检查代码质量。任何警告都会导致 CI 失败。不可避免的警告请用 `#[allow]` 或 `#[expect]` 注明原因。
- **Unsafe**：尽量避免 `unsafe`。如必须使用，请添加注释解释原因。
- **错误处理**：
  - `maa-cli` 使用 `anyhow`。
  - `maa-sys` 使用 `thiserror`。
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

## 提交规范

- 一般通过 PR 合并代码，避免直接向主分支提交。
- PR 默认使用 squash 合并，PR 内 commit message 不做限制。
- PR 标题和描述需遵循 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/) 标准。

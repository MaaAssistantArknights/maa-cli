# 安装及编译

maa-cli 提供多种方式安装，包括包管理器、预编译二进制文件和通过 `cargo` 自行编译安装。

## 通过包管理器安装

对于 macOS 和受支持的 Linux 发行版用户，推荐使用包管理器安装 maa-cli。

### macOS

Homebrew 用户可以通过非官方的 [tap](https://github.com/MaaAssistantArknights/homebrew-tap/) 安装 maa-cli：

- 稳定版本：

  ```bash
  brew install MaaAssistantArknights/tap/maa-cli
  ```

- 不稳定预发行版本：

  ```bash
  brew install MaaAssistantArknights/tap/maa-cli-beta
  ```

### Linux

- Arch Linux 用户可以安装 [AUR 包](https://aur.archlinux.org/packages/maa-cli/)：

  ```bash
  yay -S maa-cli
  ```

- ❄️ Nix 用户可以直接运行:

  ```bash
  # 稳定版
  nix run nixpkgs#maa-cli
  ```

  稳定版打包至 [nixpkgs](https://github.com/NixOS/nixpkgs/blob/nixos-unstable/pkgs/by-name/ma/maa-cli/package.nix) 中，使用 `nixpkgs` 中的 Rust 工具链。

- 对于在 Linux 上使用 Homebrew 的用户，参见上述 macOS 的安装方式。

### Windows

对于 Windows 用户，可以使用 [winget](https://github.com/microsoft/winget-pkgs/tree/master/manifests/m/MaaAssistantArknights/maa-cli/) 管理 maa-cli：
  
- 安装

  ```bash
  winget install maa-cli
  ```

- 升级

  ```bash
  winget update maa-cli
  ```

- 卸载

  ```bash
  winget uninstall maa-cli
  ```

你可以使用参数 `--purge` 来删除所有配置文件和数据文件。

## 预编译二进制文件

如果你的系统不受支持或者不想使用包管理器，你可以使用安装脚本。

**Linux 和 macOS：**

```bash
curl -fsSL https://raw.githubusercontent.com/MaaAssistantArknights/maa-cli/main/install.sh | bash
```

**Windows (PowerShell)：**

```powershell
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/MaaAssistantArknights/maa-cli/main/install.ps1" -OutFile install.ps1; .\install.ps1
```

### 支持平台

预编译二进制文件按测试程度分为三个等级：

- Tier 1：每个 PR 都会构建并运行完整测试，包括依赖 MaaCore 的测试；
- Tier 2：每个 PR 都会构建并测试，但不运行依赖 MaaCore 的测试；
- Tier 3：仅在发布时构建，不保证测试通过。

| 平台                | Target                      | 等级   |
| ------------------- | --------------------------- | ------ |
| Linux x86_64        | `x86_64-unknown-linux-gnu`  | Tier 1 |
| Linux aarch64       | `aarch64-unknown-linux-gnu` | Tier 1 |
| macOS Apple Silicon | `aarch64-apple-darwin`      | Tier 1 |
| Windows x86_64      | `x86_64-pc-windows-msvc`    | Tier 1 |
| Windows ARM64       | `aarch64-pc-windows-msvc`   | Tier 2 |
| macOS Intel         | `x86_64-apple-darwin`       | Tier 3 |

## Android (Termux)

Android 用户可以在 [Termux](https://termux.dev/) 中使用预编译的包。包内只包含交叉编译的 maa-cli、连接脚本和 Android 专用的 ncnn OCR 权重；MaaCore 与资源在安装后用 `maa install` 从官方 Release 获取，因此 core 的发版不必等 maa-cli 重新打包。下载对应架构（`aarch64-linux-android` 或 `x86_64-linux-android`）的压缩包解压后，按以下步骤安装：

```bash
# 依赖：adb，MaaCore 通过它连接游戏
pkg install -y android-tools

# maa-cli 在 Android 上走不通系统的目录约定，用环境变量指定数据目录
echo 'export MAA_DATA_DIR=$HOME/.local/share' >> ~/.bashrc
. ~/.bashrc

mkdir -p ~/.local/bin "$MAA_DATA_DIR"
cp bin/maa ~/.local/bin/ && chmod +x ~/.local/bin/maa
cp -r share/maa/ncnn-overlay "$MAA_DATA_DIR"/

# 安装 MaaCore 与资源；装完会把包内的 ncnn OCR 权重叠加进资源目录
maa install
```

如果 `~/.local/bin` 不在 `PATH` 中：

```bash
echo 'export PATH=$HOME/.local/bin:$PATH' >> ~/.bashrc && . ~/.bashrc
```

::: tip 更新 MaaCore

Android 上更新 core 时加 `--no-resource`，只换 core 不动资源：

```bash
maa update --no-resource
```

包内的 ncnn 权重是数据的预处理产物，无法在设备上生成，只与打包时那份资源同源。更新资源会让权重与新的 onnx 脱配，届时 maa-cli 检测到版本戳不一致会跳过叠加并给出提示，OCR 随之不可用，需要等带新权重的 maa-cli 包。确实要更新资源时直接跑 `maa update`。

:::

### 在 Android 上连接游戏

Termux 无法使用 Android 原生控制，只能通过 adb 连接游戏，原因见[常见问题](faq.md)。

包内附带的 `maa-android-connect.sh` 会按成功率从高到低依次尝试 mDNS 发现、root 设固定端口和扫描常见端口：

```bash
./maa-android-connect.sh
```

也可以只尝试其中一种方式：

```bash
./maa-android-connect.sh --root      # 只试 root 方式
./maa-android-connect.sh --mdns      # 只试 mDNS
./maa-android-connect.sh --verbose   # 查看详细过程
```

全部失败时脚本会提示查看包内 `README.md` 中的手动步骤，其内容为：

```bash
# 无 root：使用系统「无线调试」（需要配对码）
adb pair 127.0.0.1:<配对端口>
adb connect 127.0.0.1:<调试端口>

# 有 root：直接设置固定端口，无需配对
su -c 'setprop service.adb.tcp.port 5555; setprop ctl.restart adbd'
adb connect 127.0.0.1:5555
```

## 编译安装

Rust 开发者可以通过 `cargo` 自行编译安装 maa-cli：

- 稳定版本：

  ```bash
  cargo install maa-cli --git https://github.com/MaaAssistantArknights/maa-cli.git --bin maa --tag stable --locked
  ```

- 开发版本：

  ```bash
  cargo install maa-cli --git https://github.com/MaaAssistantArknights/maa-cli.git --bin maa --locked
  ```

### 编译选项

从源码编译时，你可以通过 `--no-default-features` 禁用默认的特性，然后通过 `--features` 来启用特定的特性。目前可用的特性有：

- `cli_installer`: 启用 `maa self update` 命令，用于更新自身，这个特性默认启用；
- `core_installer`: 启用 `maa install` 和 `maa update` 命令，用于安装和更新 MaaCore 及资源，这个特性默认启用；
- `git2`: 提供 `libgit2` 资源更新后端，这个特性默认启用；
- `vendored-openssl`: 自行编译 `openssl` 库，而不是使用系统的 `openssl` 库，这个特性默认禁用，这个特性通常在你的系统没有安装 `openssl` 库或者 `openssl` 版本过低时启用。

## 生成补全脚本

::: tip

对于使用包管理器安装的用户，这通常是自动进行的，请先检查是否已经有补全提示了。

:::

你可以使用下面的命令为你的 shell 生成补全脚本：

```bash
# 支持动态补全，例如动态补全 run 命令的任务列表
env MAA_COMPLETE=<shell> maa

# 或使用静态补全
maa complete <shell>
```

其中 `<shell>` 可以是 `bash`、`zsh`、`fish`、`powershell` 或 `elvish`。

## 安装 MaaCore 及资源

maa-cli 只提供了一个命令行界面，它需要 MaaCore 和资源来运行任务。一旦 maa-cli 安装完成，你可以通过它安装 MaaCore 及资源：

```bash
maa install
```

对于 Windows 平台用户，在运行 `maa install` 命令前，请以管理员身份在命令提示符或PowerShell中运行以下命令，以安装必要工具组VC++

- Windows:

  ```bat
  winget install "Microsoft.VCRedist.2015+.x64" --override "/repair /passive /norestart" --uninstall-previous --accept-package-agreements --force
  ```

对于使用包管理器安装的用户，可以通过包管理器安装 MaaCore：

- Homebrew：

  ```bash
  brew install MaaAssistantArknights/tap/maa-core
  ```

- Arch Linux：

  ```bash
  yay -S maa-assistant-arknights
  ```

- Nix：

  Nix 上的 maa-cli 强制依赖 MaaCore。 因此，Nix 用户无需，也不应该手动安装 MaaCore。

**注意**：只有使用包管理器安装 maa-cli 的用户才能使用包管理器安装 MaaCore，否则请使用 `maa install` 命令安装。此外，`maa install` 通过下载官方预编译的 MaaCore，而包管理器安装的 MaaCore 可能使用的编译选项和依赖版本与官方预编译的版本不同，这可能导致性能和功能上存在略微差异。

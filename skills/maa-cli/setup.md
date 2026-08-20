# 安装与运行时

在用户本机把 maa-cli 接到能打的明日方舟客户端。
命令示例写 `maa`；winget 安装后改成 `maa-cli`。
技能未写到的安装细节以 [install.md](https://github.com/MaaAssistantArknights/maa-cli/blob/main/crates/maa-cli/docs/zh-CN/install.md) 和 [config.md](https://github.com/MaaAssistantArknights/maa-cli/blob/main/crates/maa-cli/docs/zh-CN/config.md) 为准。

完成标准：`maa version` 含 Core；`maa dir library` 下能找到 MaaCore 动态库；profile 的 `preset` / 地址与当前客户端一致；目标任务 `--dry-run` 通过。

## Contents

- [1. 安装 maa-cli](#1-安装-maa-cli)
- [2. 安装 MaaCore 与资源](#2-安装-maacore-与资源)
- [3. 游戏运行时（按平台）](#3-游戏运行时按平台)
- [4. 写连接 profile](#4-写连接-profile)
- [5. 连通性检查](#5-连通性检查)

## 1. 安装 maa-cli

探测：`uname -s`（或 Windows 的 `$env:OS`），以及 `command -v maa` / `command -v maa-cli`。

### macOS

推荐 Homebrew tap：

```bash
brew install MaaAssistantArknights/tap/maa-cli
```

预发行版用 `maa-cli-beta`。Linux 上的 Homebrew 同样走这个 tap。

### Linux

- Arch：`yay -S maa-cli`
- Nix：`nix run nixpkgs#maa-cli`（已依赖 MaaCore，跳过第 2 步的 `maa install`）

### Windows

```bat
winget install maa-cli
```

此后命令名是 `maa-cli`，不是 `maa`。升级用 `winget update maa-cli`。

### 脚本 / 源码

系统不受包管理器支持时：

```bash
curl -fsSL https://raw.githubusercontent.com/MaaAssistantArknights/maa-cli/main/install.sh | bash
```

Windows PowerShell：

```powershell
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/MaaAssistantArknights/maa-cli/main/install.ps1" -OutFile install.ps1; .\install.ps1
```

Rust 开发者：`cargo install maa-cli --git https://github.com/MaaAssistantArknights/maa-cli.git --bin maa --tag stable --locked`

国内访问 GitHub 失败时，让用户改 `$MAA_CONFIG_DIR/cli.toml` 里的 `api_url` / `download_url` 镜像；镜像字段说明在 config.md 的「CLI 相关配置」。

## 2. 安装 MaaCore 与资源

maa-cli 只是 CLI。没有 Core 就不能跑任务。

```bash
maa install
```

已装则 `maa update`。热更新资源：`maa hot-update`。即使开了热更新，基础资源仍要随 Core 安装。

Windows：在 `install` 之前先装 VC++ 运行库：

```bat
winget install "Microsoft.VCRedist.2015+.x64" --override "/repair /passive /norestart" --uninstall-previous --accept-package-agreements --force
```

包管理器可选 Core（仅当 maa-cli 也是同一包管理器装的）：

- Homebrew：`brew install MaaAssistantArknights/tap/maa-core`
- Arch：`yay -S maa-assistant-arknights`

`maa install` 下的是官方预编译 Core；包管理器编出来的可能略有差异。
Core 通道 `Alpha` 只在 Windows 可用。

确认：`maa version` 同时列出 maa-cli 与 MaaCore；`maa dir library`、`maa dir resource` 非空。

## 3. 游戏运行时（按平台）

maa-cli 只负责连上已有客户端。游戏、模拟器、PlayCover 都要用户（或你按官方文档协助用户）另行安装。

### macOS：PlayCover + PlayTools（iOS 包）

适用：Apple Silicon 上用 iOS 版明日方舟，追求原生流畅。Intel Mac 更常见的是 Android 模拟器，见下一节。

MaaCore 这边：profile 设 `preset = "PlayCover"`。maa-cli 会：

- 忽略 `adb_path`
- 默认连 `127.0.0.1:1717`（可用 `address` 或 `--addr` 覆盖）
- 强制 `touch_mode = MacPlayTools`
- 加载 iOS 差分资源
- 在 `startup` 且需要拉起游戏时，用 `open` 打开 PlayCover 容器里的 `.app`（官服 bundle `com.hypergryph.arknights`）。B 服没有 App Store / PlayCover 包。

客户端与 PlayTools 不由 maa-cli 安装。按 [MAA macOS 手册 · PlayCover](https://maa.plus/docs/zh-cn/manual/device/macos.html) 做完这些，再让 maa-cli 去连：

1. 安装手册指定的 PlayCover（MAA 要求用其文档中的 fork，不要假设任意 PlayCover 发行版都能用）。
2. 在 PlayCover 里安装 iOS 版明日方舟。游戏更新后通常要重新装包。
3. PlayCover 右键游戏 → 设置 → 绕过：启用 PlayChain、绕过越狱检测、插入内省库、**MaaTools**（即 PlayTools）。
4. 启动游戏。标题栏出现 `[localhost:端口]` 才算 PlayTools 起来了。把该地址写入 profile（常见 `127.0.0.1:1717`）。
5. 识别出错时，在 PlayCover 里把分辨率设到 1080p。
6. 运行期间保持窗口可见：最小化、台前调度切走、移到其它桌面会导致截图失败。

交互 `maa init` 选预设 `PlayCover` 时，地址填 PlayTools 的 `host:port`。

### macOS：Android 模拟器

PlayCover 不是唯一路径。macOS 也可以 ADB 连模拟器，此时用 `ADB` / `MaaTouch`，不用 `MacPlayTools`。

|模拟器|maa-cli preset|备注|
|---|---|---|
|MuMu Player Pro|`MuMuPro`|默认 adb：`/Applications/MuMuPlayer.app/Contents/MacOS/MuMuEmulator.app/Contents/MacOS/tools/adb`；默认 `127.0.0.1:16384`|
|BlueStacks Air|`ADB`|开「Android 调试（ADB）」；测试可用 MaaTouch + `127.0.0.1:5555`|
|Android Studio AVD|`ADB`|自带 platform-tools；Android 10+ 上 MiniTouch 在 SELinux Enforcing 下不可用，改 MaaTouch/ADB|
|夜神（Intel）|`ADB`|adb 常在 `/Applications/NoxAppPlayer.app/Contents/MacOS/adb`|

模拟器必须已启动、ADB 开关已打开。用模拟器自带 `adb devices` 核对 serial，再写入 `address`。

### Windows：模拟器 + ADB

常见路径：模拟器 + ADB。`maa init` 可选 `Androws`（腾讯 Androws，默认 `127.0.0.1:5555`）或通用 `ADB`。

- 优先使用模拟器自带 adb，把 `adb_path` 指到那个可执行文件。
- 端口见 [连接说明](https://maa.plus/docs/zh-cn/manual/connection.html)。
- 触控：兼容优先 `ADB`；多数模拟器可用 `MaaTouch`。`MacPlayTools` 只给 PlayCover。

### Linux：Waydroid / AVD

- **Waydroid**：profile `preset = "Waydroid"`。仍需本机 `adb`（默认 PATH 里的 `adb`）。maa-cli 会 `waydroid status`，必要时起 session，再 `waydroid adb connect`。分辨率至少 720p 且 16:9，例如：

  ```bash
  waydroid prop set persist.waydroid.width 1280
  waydroid prop set persist.waydroid.height 720
  ```

  细节：[MAA Linux 手册 · Waydroid](https://maa.plus/docs/zh-cn/manual/device/linux.html)

- **AVD / 其它**：`preset = "ADB"`，`adb_path` 指向 SDK `platform-tools/adb` 或发行版 `adb`。同样要求 16:9 且 ≥720p。

Linux 上跑 MAA 桌面 GUI（Wine）与 maa-cli 是两条线；命令行任务走 maa-cli + 原生 Core。

## 4. 写连接 profile

目录：`$(maa dir config)/profiles/`。默认名 `default`，扩展名 json/toml/yaml 均可。本次运行换文件：`--profile 名字`。

`maa init` 会交互询问预设（`MuMuPro` / `PlayCover` / `Waydroid` / `Androws` / `ADB`）、adb 路径、地址、触控模式。代理无人值守时直接写文件。

PlayCover：

```toml
[connection]
preset = "PlayCover"
address = "127.0.0.1:1717"

[instance_options]
touch_mode = "MacPlayTools"
```

`touch_mode` 即使写错，PlayCover 预设也会被强制改成 `MacPlayTools`。

Waydroid：

```toml
[connection]
preset = "Waydroid"
```

通用 ADB（按实际 serial 改 `address`）：

```toml
[connection]
preset = "ADB"
adb_path = "adb"
address = "127.0.0.1:5555"

[instance_options]
touch_mode = "MaaTouch"
```

非简中客户端：在 `startup` 里带 `client_type`，或在 profile 里设 `resource.global_resource`（`YoStarEN` / `YoStarJP` / `YoStarKR` / `txwy`）。iOS / PlayCover 会自动加 `platform_diff_resource = "iOS"`。

## 5. 连通性检查

按顺序排除，不要跳着改。

1. `maa version`：没有 Core → 回到第 2 步。
2. 游戏或模拟器窗口在前台（PlayCover 尤其不能最小化）。
3. PlayCover：标题栏有 `[localhost:port]`，profile 地址与之一致。ADB：`"$adb_path" devices` 能看到 `device` 而不是 `offline`。
4. `maa startup --dry-run` 通过后再去掉 `--dry-run`。需要拉起游戏时：`maa startup Official`（或对应 `client_type`）。
5. 仍失败：`-v` 或 `MAA_LOG=debug`，日志目录 `maa dir log`。

常见失败：

- 连 `emulator-5554` 但模拟器实际是 `127.0.0.1:16384` 这类端口 → 改 `address` 或 `--addr`。
- macOS 默认连 ADB，游戏却在 PlayCover 里 → 改 preset。
- 多设备时 ADB 预设会拿 `adb devices` 的第一台 → 显式写 `address`。
- MiniTouch + 新 Android 模拟器 → 改 `MaaTouch` 或 `ADB`。

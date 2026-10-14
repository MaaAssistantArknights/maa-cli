# 常见问题

## 1. 如何在 macOS 上使用 `$HOME/.config/maa` 作为配置文件目录？

由于 Rust 库 [Directories](https://github.com/dirs-dev/directories-rs/) 在 macOS 上默认使用 Apple 风格目录，maa-cli 默认也使用 Apple 风格的配置目录。但是对于命令行程序来说，XDG 风格的目录更加合适。如果你想要使用 XDG 风格目录，你可以设置 `XDG_CONFIG_HOME` 环境变量，如 `export XDG_CONFIG_HOME="$HOME/.config"`，这会让 maa-cli 使用 XDG 风格配置目录。如果你想要使用 XDG 风格配置目录，但是不想设置环境变量，你可以使用下面的命令创建一个符号链接：

```bash
mkdir -p "$HOME/.config/maa"
ln -s "$HOME/.config/maa" "$(maa dir config)"
```

## 2. 为什么 Android/Termux 上只能通过 adb 连接游戏？

MaaCore 通过两种方式控制游戏：Android 原生控制和 adb。原生控制要求 MaaCore 通过 `libbridge.so` 提供截图与输入注入，而官方的 `libbridge.so` 是 Android App 的 JNI 桥，依赖 MediaProjection（虚拟显示）和 AccessibilityService，需要运行在 Android App 中。Termux 是纯 Linux 进程环境，没有这些能力，因此只能通过 adb 连接。这也是 Android 包内不含 `libMaaAndroidNativeControlUnit.so` 的原因。

安装与连接的具体步骤见[安装及编译](install.md)。

## 3. Android 上有哪些已知限制？

- 必须通过 adb 连接游戏（见上一个问题），稳定性和性能受 adb 回环影响；
- 不支持 `git2` 资源后端：Android 包以 `--no-default-features --features core_installer` 构建（`git2` 的 `openssl` 依赖无法在 Android 上交叉编译），因此 `maa update` 通过 HTTP 而非 git 拉取资源，功能可用；
- 不要在 Android 上执行 `maa install` 或 `maa update` 更新 MaaCore 及资源，包内已包含适配的版本，详见[安装及编译](install.md)；
- MAA 只支持 16:9 分辨率，识别异常时先用 `wm size` 查看当前分辨率，必要时用 `wm size 1080x1920` 调整（部分设备需要 root）；
- 如果官方 MAA-Meow 已经能满足需求，可以优先考虑它（原生控制、免 adb、带图形界面）。

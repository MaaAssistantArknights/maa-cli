# Install and Build

There are multiple ways to install maa-cli, including package managers, precompiled binaries, and building from source with `cargo`.

## Install via Package Manager

For macOS and supported Linux distributions, it is recommended to install maa-cli using a package manager.

### macOS

Homebrew users can install maa-cli via the unofficial [tap](https://github.com/MaaAssistantArknights/homebrew-tap/):

- Stable release:

  ```bash
  brew install MaaAssistantArknights/tap/maa-cli
  ```

- Beta releases:

  ```bash
  brew install MaaAssistantArknights/tap/maa-cli-beta
  ```

### Linux

- Arch Linux users can install the [AUR package](https://aur.archlinux.org/packages/maa-cli/):

  ```bash
  yay -S maa-cli
  ```

- ❄️ Nix users can run directly:

  ```bash
  # Stable release
  nix run nixpkgs#maa-cli
  ```

  The stable release is packaged in [nixpkgs](https://github.com/NixOS/nixpkgs/blob/nixos-unstable/pkgs/by-name/ma/maa-cli/package.nix) and uses the nixpkgs Rust toolchain.

- Users using Homebrew on Linux please refer to the macOS installation method above.

## Precompiled Binaries

If package managers are not available on your system or you prefer not to use them, you can use the installation scripts.

**Linux and macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/MaaAssistantArknights/maa-cli/main/install.sh | bash
```

**Windows (PowerShell):**

```powershell
Invoke-WebRequest -Uri "https://raw.githubusercontent.com/MaaAssistantArknights/maa-cli/main/install.ps1" -OutFile install.ps1; .\install.ps1
```

## Build from Source

Rust developers can compile and install maa-cli themselves via `cargo`:

- Stable version:

  ```bash
  cargo install maa-cli --git https://github.com/MaaAssistantArknights/maa-cli.git --bin maa --tag stable --locked
  ```

- Development version:

  ```bash
  cargo install maa-cli --git https://github.com/MaaAssistantArknights/maa-cli.git --bin maa --locked
  ```

### Features

When compiling from source, you can disable the default features with `--no-default-features` and then enable specific features with `--features`. The available features are:

- `cli_installer`: Provide `maa self update` command to update self, this feature is enabled by default;
- `core_installer`: Provide `maa install` and `maa update` commands to install and update MaaCore and resources, this feature is enabled by default;
- `git2`: Provide `libgit2` resource backend, this feature is enabled by default;
- `vendored-openssl`: Build OpenSSL library by self instead of using system library, this feature is disabled by default;

## Generate Completion Script

::: tip

It is usually done automatically for users who installed via package managers, please check if you already have completion hints first.

:::

You can use the following command to generate completion scripts for your shell:

```bash
# Support dynamic completion, such as the task list of the run subcommand
env MAA_COMPLETE=<shell> maa

# Or using static completion
maa complete <shell>
```

Where `<shell>` can be `bash`, `zsh`, `fish`, `powershell`, or `elvish`.

## Install MaaCore

maa-cli only provides an interface for MaaCore, it needs MaaCore and resources to run tasks, which can be installed by maa-cli once it is installed:

```bash
maa install
```

For Windows platform users, before running the `maa install` command, please run the following command as administrator in Command Prompt or PowerShell to install the required VC++ toolset:

- Windows:

  ```bat
  winget install "Microsoft.VCRedist.2015+.x64" --override "/repair /passive /norestart" --uninstall-previous --accept-package-agreements --force
  ```

For users who installed via package managers, MaaCore can also be installed via package managers:

- Homebrew：

  ```bash
  brew install MaaAssistantArknights/tap/maa-core
  ```

- Arch Linux：

  ```bash
  yay -S maa-assistant-arknights
  ```

- Nix：

  maa-cli on Nix depends on the MaaCore package, so no additional installation is required.

**NOTE**: Only users who installed maa-cli via package managers can install MaaCore via package managers. Otherwise, please use the `maa install` command to install. In addition, the `maa install` downloads the official precompiled MaaCore, while the MaaCore installed by package managers has different compilation options and dependency versions from the official precompiled version, potentially causing variations in behavior and performance.

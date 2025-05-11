<!-- markdownlint-disable MD033 MD041 -->
<div align="center">

# maa-cli

![CI](https://img.shields.io/github/actions/workflow/status/MaaAssistantArknights/maa-cli/ci.yml?logo=GitHub&label=CI)
![Codecov](https://img.shields.io/codecov/c/github/MaaAssistantArknights/maa-cli?logo=codecov)
<br>
![License](https://img.shields.io/badge/license-AGPL--3.0--only-blueviolet)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blueviolet)
<br>
![Stable Release Version](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fgithub.com%2FMaaAssistantArknights%2Fmaa-cli%2Fraw%2Fversion%2Fstable.json&query=%24.version&prefix=v&label=stable)
![Beta Release Version](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fgithub.com%2FMaaAssistantArknights%2Fmaa-cli%2Fraw%2Fversion%2Fbeta.json&query=%24.version&prefix=v&label=beta)
![Nightly Release Version](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fgithub.com%2FMaaAssistantArknights%2Fmaa-cli%2Fraw%2Fversion%2Falpha.json&query=%24.version&prefix=v&label=nightly)

[简体中文](crates/maa-cli/docs/zh-CN/intro.md)
| [English](crates/maa-cli/docs/en-US/intro.md)
| [한국어](crates/maa-cli/docs/ko-KR/intro.md)

一个使用 Rust 编写的 [MAA][maa-home] 命令行工具。

</div>

<!-- markdownlint-enable MD033 MD041 -->

由于本项目当前维护者不具备 Windows 环境，对 Windows 平台及相关功能的支持较为有限，**因此我们非常欢迎并迫切需要对 Windows 相关部分的贡献**。

## 功能

- 运行预定义或自定义的任务，例如 `maa fight`，`maa run <task>`;
- 使用 `maa install` 和 `maa update` 安装和更新MaaCore及资源；
- 使用 `maa self update` 更新自身。

## 文档

- [安装及编译](crates/maa-cli/docs/zh-CN/install.md)
- [使用](crates/maa-cli/docs/zh-CN/usage.md)
- [配置](crates/maa-cli/docs/zh-CN/config.md)
- [常见问题](crates/maa-cli/docs/zh-CN/faq.md)

[maa-home]: https://github.com/MaaAssistantArknights/MaaAssistantArknights/

## 许可证

本项目使用 [AGPL-3.0-only](LICENSE) 许可证。相关第三方项目的许可证请参阅 [licenses](licenses.md)。

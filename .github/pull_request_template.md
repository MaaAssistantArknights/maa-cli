<!-- markdownlint-disable-file MD041 -->
<!--
提交前请先阅读贡献指南 / Please read the contributing guide first:
https://github.com/MaaAssistantArknights/maa-cli/blob/main/CONTRIBUTING.md
-->

## 动机 / Motivation

<!--
为什么要改，解决什么问题。有相关 issue 请写 `Closes #123`。新功能、新命令、新配置请先开 issue 讨论。
Why is this change needed? What problem does it solve? Link the issue if there is one, e.g. `Closes #123`. Discuss new features, commands and config options in an issue first.
-->

## 方案 / Solution

<!--
怎么解决的。重点写从 diff 里看不出来的设计取舍。
How does this PR solve it? Focus on design choices that are not obvious from the diff.
-->

## 变更与影响 / Changes and impact

<!--
用户能感受到什么变化：命令行行为、配置格式、平台支持等。breaking change 请明确写出。纯内部改动写“无”。
What will users notice: CLI behavior, config format, platform support? Call out breaking changes explicitly. Write "None" for internal-only changes.
-->

## 验证 / Testing

<!--
实际运行了哪些命令，在什么平台上验证过，是否连接真实设备或客户端测试过。不用贴测试通过的数量。
Which commands did you actually run, on which platforms? Did you test against a real device or client? No need to paste test counts.
-->

- [ ] 本 PR 只包含实现上述目的所需的改动，无关的修复或重构已拆出 / This PR only contains changes needed for the purpose above; unrelated fixes and refactors are split out
- [ ] 改了用户可见行为的话，已同步更新 `crates/maa-cli/docs/` 下所有语言的文档 / User-visible behavior changed: docs under `crates/maa-cli/docs/` are updated for every language
- [ ] 如果是 breaking change，目标分支是 `dev-0.8` 而不是 `main` / Breaking change: this PR targets `dev-0.8`, not `main`

## AI 辅助 / AI assistance

<!--
如果用了 AI，请写明工具（harness）、模型和参与程度；没用就写“无”。
If AI was used, name the tool (harness), the model, and how much it contributed. Write "None" otherwise.
参与程度 / Extent: 大部分由 AI 生成 / mostly generated; 部分辅助 / partly assisted; 仅用于审查 / review only
未经作者本人审阅和整理的 AI 生成 PR 可能会被直接关闭。
AI-generated PRs that the author has not reviewed and cleaned up may be closed without review.
-->

- 工具 / Harness:
- 模型 / Model:
- 参与程度 / Extent:
- [ ] 如果用了 AI，我已亲自审阅并整理全部改动，能解释每一处修改 / If AI was used, I have personally reviewed and cleaned up all changes and can explain each of them

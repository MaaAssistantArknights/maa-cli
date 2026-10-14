<!-- markdownlint-disable-file MD041 -->
<!--
提交前请先阅读贡献指南 / Please read the contributing guide first:
https://github.com/MaaAssistantArknights/maa-cli/blob/main/CONTRIBUTING.md
如果是较小的改动，填写「动机」一节即可，其余部分可以按需删除。
For small changes, filling in the Motivation section is enough; feel free to remove the rest.
-->

## 动机 / Motivation

<!--
为什么要改，解决什么问题。有相关 issue 请写 `Closes #123`。新功能、新命令、新配置请先开 issue 讨论。
Why is this change needed? What problem does it solve? Link the issue if there is one, e.g. `Closes #123`. Discuss new features, commands and config options in an issue first.
-->

## 方案 / Solution

<!--
怎么解决的。重点写从 diff 里看不出来的设计取舍，以及哪里你不确定。
How does this PR solve it? Focus on design choices that are not obvious from the diff, and on what you are unsure about.
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

## AI 辅助 / AI assistance

<!--
勾选其中一项。使用了 AI 时，请写明哪些部分由 AI 编写，如全部代码、只有测试、只有文档；也可以注明工具和模型。
Check one. If AI was used, say which parts it wrote, e.g. all code, tests only, docs only; naming the tool and model is optional.
-->

- [ ] 未使用 AI / No AI used
- [ ] 使用了 AI，我已亲自审阅并整理全部改动，能解释每一处修改 / AI was used, and I have personally reviewed and cleaned up all changes and can explain each of them
  - AI 编写的部分 / AI-written parts:

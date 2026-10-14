# 贡献指南 / Contributing

开发环境、代码规范和提交流程见 [开发文档](DEVELOPMENT.md)。本文只写 PR 的基本要求，尤其是使用 AI 工具时的要求。

## 先讨论，再写代码

- 新功能、新命令、新配置项或配置格式的改动，请先开 issue 讨论方案，达成一致后再写代码。没有讨论过的大型 PR 很可能需要推倒重来。
- 提交前先搜一下有没有相同主题的 issue 或 PR，避免重复工作。

## 一个 PR 只做一件事

- PR 只包含实现目的所需的改动。顺手发现的问题、无关的重构请单独提 PR。
- 改动较大时，拆成能独立 review 的几个提交，或者拆成多个 PR。
- PR 描述保持简短，写清楚为什么改、怎么改、实际怎么验证的。不要粘贴工具生成的长篇摘要。

## 使用 AI 工具

欢迎使用 AI 工具，但作者要对提交的每一行负责。

- 让你的 agent 先读 [AGENTS.md](AGENTS.md)，它包含本仓库的代码规范和检查步骤。
- 提交前亲自审阅并整理全部改动：删掉没有被要求的功能、多余的注释和防御性代码，确保你能解释每一处修改。
- 在 PR 模板的「AI 辅助」一节写明使用的工具、模型和参与程度。
- 回复 review 时请用自己的话，不要直接转贴 AI 的输出。
- 明显没有经过作者审阅的大型 AI 生成 PR，维护者可能会直接关闭。长时间不回应 review 的 PR 也可能会被关闭。

## English

See [DEVELOPMENT.md](DEVELOPMENT.md) for the development setup, code conventions and workflow. This file covers what we expect from a PR, especially when AI tools are involved.

- **Discuss first.** Open an issue to agree on the design before implementing a new feature, command, config option or config format change. Large PRs without prior discussion will likely need to be redone. Search existing issues and PRs to avoid duplicate work.
- **One PR, one purpose.** Include only the changes needed for that purpose; send unrelated fixes and refactors separately. Keep the description short: why, how, and how you actually verified it. Do not paste long tool-generated summaries.
- **AI tools are welcome, but you own every line.** Have your agent read [AGENTS.md](AGENTS.md) first. Review and clean up all changes yourself before submitting: remove features nobody asked for, excessive comments and unnecessary defensive code, and make sure you can explain every change. Fill in the "AI assistance" section of the PR template. Reply to reviews in your own words instead of pasting AI output.
- Maintainers may close large AI-generated PRs that were clearly not reviewed by their author, as well as PRs that stay unresponsive to review for a long time.

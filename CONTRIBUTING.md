# 贡献指南 / Contributing

开发环境、代码规范和提交流程见 [开发文档](DEVELOPMENT.md)。本文是提交 PR 的规则，无论是否使用 AI 工具都适用。

## 规则

1. **先讨论，再写代码。** 新功能、新命令、新配置项、配置格式变更，以及预计超过约 500 行的改动，请先开 issue 讨论方案，得到维护者认可后再写代码。bug 修复、文档和小改进可以直接提 PR。动手前先搜一下有没有相同主题的 issue 或 PR。
2. **一个 PR 只做一件事。** 只包含实现目的所需的改动。顺手发现的问题、无关的重构请单独提。
3. **作者对每一行负责。** 欢迎使用 AI 工具，但提交前请亲自审阅并整理全部改动，确保能解释每一处修改。删掉没有被要求的功能、多余的注释和不必要的防御性代码。使用 agent 时请让它先读 [AGENTS.md](AGENTS.md)。
4. **描述简短具体。** 按 PR 模板写清楚为什么改、怎么改、实际怎么验证的，以及哪里你不确定。不要粘贴工具生成的长篇摘要。
5. **用自己的话回复 review。** 不要直接转贴 AI 的输出。

## 维护者会怎么处理

- 违反第 1 条：请你先开 issue 讨论，PR 可能会先关闭，方案确定后再重新提交。
- 违反第 2 条或改动过大：请你拆分，不拆分的 PR 可能会被关闭。
- 明显没有经过作者审阅的 AI 生成 PR：可能会被直接关闭，不做逐行 review。
- review 后 30 天没有回应：可能会被关闭，之后随时可以重新打开。

这些处理不针对个人，只是为了让 review 的精力用在能合并的改动上。

## English

See [DEVELOPMENT.md](DEVELOPMENT.md) for the development setup, code conventions and workflow. The rules below apply to every PR, whether or not AI tools were used.

1. **Discuss first.** Open an issue and get the maintainers' agreement on the design before implementing a new feature, command, config option, config format change, or anything expected to exceed roughly 500 lines. Bug fixes, docs and small improvements can go straight to a PR. Search existing issues and PRs before starting.
2. **One PR, one purpose.** Include only the changes needed for that purpose. Send unrelated fixes and refactors separately.
3. **You own every line.** AI tools are welcome, but review and clean up all changes yourself before submitting, and make sure you can explain each of them. Remove features nobody asked for, excessive comments and unnecessary defensive code. If you use an agent, have it read [AGENTS.md](AGENTS.md) first.
4. **Keep the description short and concrete.** Follow the PR template: why, how, how you actually verified it, and what you are unsure about. Do not paste long tool-generated summaries.
5. **Reply to reviews in your own words.** Do not paste AI output.

What maintainers will do:

- Rule 1 not followed: we will ask you to open an issue first, and the PR may be closed until the design is agreed.
- Rule 2 not followed, or the PR is too large: we will ask you to split it; PRs that are not split may be closed.
- AI-generated PRs that were clearly not reviewed by their author may be closed without a line-by-line review.
- No response for 30 days after a review: the PR may be closed; you can reopen it at any time.

# 贡献指南 / Contributing

感谢你对 maa-cli 的贡献。开发环境、代码规范和提交流程见 [开发文档](DEVELOPMENT.md)，本文说明提交 PR 时需要注意的事项，无论是否使用 AI 工具均适用。

本项目维护者人手有限，review 往往是合并的瓶颈。以下约定的目的是让改动能够更快地被审阅和合并。

Thank you for contributing to maa-cli. See [DEVELOPMENT.md](DEVELOPMENT.md) for the development setup, code conventions and workflow. This document describes what we expect from a PR, whether or not AI tools were used. The project has few maintainers and review is often the bottleneck; these conventions help changes get reviewed and merged sooner.

## 提交 PR 前 / Before submitting a PR

1. **先在 issue 中讨论方案。** 新功能、新命令、新配置项、配置格式的变更，以及预计超过约 500 行的改动，请先开 issue 与维护者确认方案，再开始实现，以免方向不一致导致返工。bug 修复、文档和小幅改进可以直接提交 PR。开始之前，请先搜索是否已有相同主题的 issue 或 PR。

   **Discuss the design in an issue first.** For a new feature, command, config option, config format change, or a change expected to exceed roughly 500 lines, open an issue and agree on the approach with a maintainer before implementing it, to avoid rework. Bug fixes, docs and small improvements can be submitted as a PR directly. Search for existing issues and PRs on the same topic before starting.

2. **每个 PR 只解决一个问题。** 开发过程中发现的其他问题或希望进行的重构，请另开 PR。范围集中的 PR 更容易审阅，也能更快合并。

   **One PR, one purpose.** Submit other problems you find or refactors you want to make as separate PRs. Focused PRs are easier to review and merge.

3. **使用 AI 生成的改动需要经过作者本人审阅。** 提交前请完整阅读所有改动，确认每一处都是必要的，并能说明修改的理由。AI 生成的代码常包含未被要求的功能、冗长的注释或不必要的防御性代码，请在提交前删除。使用 agent 开发时，请让它先阅读 [AGENTS.md](AGENTS.md)。

   **AI-generated changes must be reviewed by the author.** Before submitting, read every change, make sure each one is necessary, and be able to explain it. AI-generated code often includes features nobody asked for, verbose comments and unnecessary defensive code; remove them before submitting. If you use an agent, have it read [AGENTS.md](AGENTS.md) first.

4. **PR 描述应简洁。** 请按照 PR 模板说明修改的原因、实现方式和实际的验证方法，如有不确定之处也请注明。不需要附上工具生成的长篇摘要。

   **Keep the description concise.** Follow the PR template: why, how, how you actually verified it, and anything you are unsure about. Do not include long tool-generated summaries.

5. **请用自己的语言回复 review。** 直接转贴 AI 的输出，会让维护者难以判断你是否认同修改意见，以及有哪些疑问。

   **Reply to reviews in your own words.** Pasted AI output makes it hard for maintainers to tell whether you agree with the feedback and what questions you have.

## 不符合约定的 PR / PRs that do not follow these conventions

维护者会在 PR 中说明原因，并视情况处理：

Maintainers will explain why in the PR and, depending on the case:

- 需要先讨论方案的：请先开 issue，PR 可能会暂时关闭，方案确定后可以重新提交。

  Needs design discussion first: you will be asked to open an issue; the PR may be closed for now and can be resubmitted once the design is agreed.

- 改动过大或包含无关内容的：请拆分为多个 PR。

  Too large or contains unrelated changes: you will be asked to split it into several PRs.

- 明显未经作者本人审阅的 AI 生成 PR：可能会直接关闭，不进行逐行 review。

  AI-generated PRs clearly not reviewed by their author: may be closed without a line-by-line review.

- review 后 30 天内没有回应的：可能会关闭，之后可以随时重新打开。

  No response within 30 days after a review: may be closed; it can be reopened at any time.

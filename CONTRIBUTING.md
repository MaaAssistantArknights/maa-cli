# 贡献指南 / Contributing

感谢你愿意为 maa-cli 贡献代码！开发环境、代码规范和提交流程请看 [开发文档](DEVELOPMENT.md)。这里说的是提交 PR 时我们希望你注意的几件事，不管有没有用 AI 工具都一样。

maa-cli 的维护者人手很少，review 的时间往往比写代码的时间更紧张。下面这些约定，都是为了让你的改动能更快、更顺利地合并。

## 提交 PR 前

1. **先在 issue 里聊聊方案。** 如果要加新功能、新命令、新配置项，或者改配置格式，或者改动预计会超过 500 行左右，请先开一个 issue，和维护者对一下思路再动手，免得写完才发现方向不对，白费功夫。修 bug、改文档和小改进可以直接提 PR。开始之前也可以搜一下，看看有没有人已经在做同样的事。
2. **一个 PR 只做一件事。** 顺手发现的其他问题或者想做的重构，欢迎另开 PR。小而专注的 PR 更容易 review，也更快能合并。
3. **用 AI 没问题，但请自己先看一遍。** 提交之前，请亲自读一遍所有改动，确认每一处都是需要的，并且你能说清楚为什么这么改。AI 常常会多加一些没人要的功能、大段注释或者用不上的防御性代码，请顺手删掉。如果用 agent 写代码，记得让它先读 [AGENTS.md](AGENTS.md)。
4. **PR 描述简短就好。** 按 PR 模板写清楚为什么改、怎么改、实际是怎么验证的，有不确定的地方也请写出来。不需要粘贴工具生成的长篇总结。
5. **review 时请用自己的话回复。** 直接转贴 AI 的回答，我们很难判断你是不是同意、哪里有疑问。

## 如果 PR 不太符合上面的约定

我们会在 PR 里说明原因，一般会这样处理：

- 需要先讨论方案的：请你先开 issue，PR 可能会暂时关闭，方案确定后欢迎重新提交。
- 改动太多、混了不相关内容的：请你拆分成几个 PR。
- 明显没有经过作者本人检查的 AI 生成 PR：我们可能会直接关闭，不再逐行 review。
- review 之后 30 天没有回应的：可能会关闭，你之后随时可以重新打开。

## English

Thanks for contributing to maa-cli! See [DEVELOPMENT.md](DEVELOPMENT.md) for the development setup, code conventions and workflow. Below is what we ask of every PR, whether or not you used AI tools. We have very few maintainers and review time is usually the bottleneck, so these help your change get merged faster.

1. **Talk about the design in an issue first.** For a new feature, command, config option, config format change, or anything likely to exceed roughly 500 lines, please open an issue and agree on the approach with a maintainer before writing code, so you don't spend effort in the wrong direction. Bug fixes, docs and small improvements can go straight to a PR. It's also worth searching for existing issues and PRs on the same topic.
2. **One PR, one purpose.** Other problems you notice or refactors you'd like to do are welcome as separate PRs. Small, focused PRs are easier to review and get merged sooner.
3. **AI is fine, but please review it yourself.** Before submitting, read through every change, make sure each one is needed, and that you can explain why it is there. AI tends to add features nobody asked for, long comments and unneeded defensive code; please remove them. If you use an agent, have it read [AGENTS.md](AGENTS.md) first.
4. **Keep the description short.** Follow the PR template: why, how, how you actually verified it, and anything you are unsure about. No need to paste long generated summaries.
5. **Reply to reviews in your own words.** When AI output is pasted as a reply, it is hard for us to tell whether you agree or what you are unsure about.

If a PR doesn't follow these, we will explain why in the PR. Usually:

- Needs design discussion first: we will ask you to open an issue; the PR may be closed for now and you are welcome to resubmit once the design is agreed.
- Too large or mixes unrelated changes: we will ask you to split it.
- AI-generated PRs clearly not reviewed by their author: may be closed without a line-by-line review.
- No response for 30 days after a review: may be closed; you can reopen it at any time.

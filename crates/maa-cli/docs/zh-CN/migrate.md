# 配置迁移

`maa migrate` 用于把已有配置迁移到当前 maa-cli 所使用的配置形式。

不同来源对应不同的子命令。本文按迁移来源分节说明；目前仅完成 WPF GUI，其他来源会在后续补充。

## 目录

- [WPF GUI](#wpf-gui)
- [config-v1 到 config-v2](#config-v1-到-config-v2)（TODO）

## WPF GUI

把 MAA WPF GUI 的配置迁移为 maa-cli 任务配置：

```bash
maa migrate wpf <input> [output] [-f toml|yaml|json] [--config <name>]
```

`<input>` 应指向 GUI 导出的配置文件：

| 平台 | 输入文件 |
| --- | --- |
| Windows | `config/gui.new.json`（相对 MAA 安装或用户配置目录；以你本机实际路径为准） |
| Linux | TODO：待补充路径与测试 |
| macOS | TODO：待补充路径与测试 |

如果文件中有多套配置，可用 `--config` 或环境变量 `MAA_GUI_CONFIG` 指定名称；未指定时会交互选择，默认使用文件中的当前配置（`Current`）。

迁移可能不完整：不支持的任务类型会被跳过；GUI 中已关闭的任务会保留在结果里，但默认不会执行；未能对应的参数会出现在迁移摘要中。命令结束后会输出这些汇总，方便你对照检查。

### StartUp

对应 GUI 任务队列中的启动任务，迁移为 maa-cli 的 `StartUp` 任务。

| GUI 字段 | maa-cli 字段 | 规则 |
| --- | --- | --- |
| `Configurations.<配置名>.Gui.RuntimeSettings.ClientType` | `tasks[].params.client_type` | 合法值（`Official` / `Bilibili` / `txwy` / `YoStarEN` / `YoStarJP` / `YoStarKR`）直接写入；缺失或不合法时将写入备选值，备选值将在 maa-cli 运行时交互确认最终值 |
| `Configurations.<配置名>.Gui.RuntimeSettings.StartGame` | `tasks[].params.start_game_enabled` | 有值时写入 |
| `Configurations.<配置名>.TaskQueue[] \| {AccountSwitchEnabled, AccountName}` | `tasks[].params.account_name` | 仅当 GUI 配置 `AccountSwitchEnabled` 为 `true` 时读取 `AccountName` 并尝试写入迁移配置 `account_name` |

TODO：`account_name` 的转换**我没有测试过**（未做实机账号切换验证）。

## config-v1 到 config-v2

TODO

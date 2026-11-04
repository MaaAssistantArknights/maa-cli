# `maa-executor` Architecture

`maa-executor` explores a runtime boundary below CLI, GUI, and remote resolvers. It accepts only resolved MaaCore inputs and does not own transport or presentation behavior.

## 原型状态 / Prototype Status

这是供设计讨论的原型，尚未接入 CLI，不代表公共 API 或运行正确性已经定稿。当前实现用于具体展示输入、资源所有权、session 编排、事件和报告之间的关系；最小调用示例位于 crate 的 rustdoc 中，并作为编译检查的 doctest。

The crate is a design prototype, not a production-ready executor. The CLI does not use it, and its public API remains provisional.

## Execution Lifecycle

```mermaid
flowchart TD
    A["Executor"] -->|build(config)| B["MaaCore runtime lease"]
    B -->|create_instance(options)| C["ExecutorInstance"]
    C -->|execute(environment, connection, sessions, stop)| D["Execution"]

    D -->|prepare| E["PreparedEnvironment"]
    E -->|resolve address and connect| F["Connected Assistant"]
    F -->|reuse instance| G["Session loop"]
    G -->|prepare session| H["PreparedSession"]
    H -->|append, start, wait, stop| G
    G -->|all sessions complete| I["ExecutionReport"]
    I -->|close environment| J["Terminal event"]

    B -->|last owner dropped| K["Unload MaaCore"]
```

`PreparedEnvironment` owns execution-level cleanup after `prepare` succeeds.
`PreparedSession` owns MaaCore task-queue cleanup from the first append attempt onward.
The Assistant is connected once and reused, while callback state is isolated by MaaCore task ID for each session.

## Event Contract

The event channel is created before the execution thread starts, so initial lifecycle events cannot be missed.
It has one receiver owned by `ExecutionHandle`; transport layers may fan out events above this crate when needed.

MaaCore callbacks are forwarded as `CoreEvent { session, kind, payload }` without presentation formatting.
Callbacks with a registered task ID carry `Some(session)`; instance-level callbacks carry `None`.

The intended contract is one terminal event after environment cleanup on normal result paths:

- `Finished` when an `ExecutionReport` is available, including failed or interrupted reports.
- `Failed` when execution ends before a report can be produced.

## 待讨论的设计问题 / Open Design Questions

1. **Instance 的职责。** 当前 `ExecutorInstance` 只保存 driver 和 options，`execute(self)` 消耗它，Assistant 在 execution 内创建。如果不需要跨 execution 复用连接，是否直接由 `Executor::execute` 接收 options 更清楚？如果需要复用，instance 应持有怎样的资源？
2. **Session 与游戏生命周期。** 当前 `Session` 是同一连接上的任务批次，environment 的 prepare/close 属于整个 execution。多账号切换、游戏启动和关闭，以及 Ctrl-C 时保留游戏的策略应由哪一层持有？关闭游戏的可选策略必须与 MaaCore 队列的必要清理分开。
3. **事件与结果的消费方式。** 当前事件接收是同步的，`wait(self)` 会释放事件接收端。GUI 或 remote 调用方如何同时消费事件和等待结果？应以实际调用示例决定是否拆分所有权或提供异步事件接口。

## 已知限制 / Known Limitations

- `auto_reconnect=false` 时，掉线 session 调用 `stop()` 后会继续下一 session，但尚未等待旧任务实际退出。MaaCore 的 `stop()` 返回并不构成工作线程完成屏障，可能使旧任务干扰下一批队列，甚至令下一 session 永久等待终态 callback。跨 session 复用的停止协议仍需验证和完善。
- 等待 terminal callback 没有超时或取消唤醒；后台线程 panic 也尚未转换为完成结果。因此唯一终态是预期协议，当前原型并未覆盖所有异常路径。
- `StopToken` 只请求停止执行，尚未实现通过 atomic policy 控制中断时是否关闭游戏。
- 现有单元测试主要覆盖 callback 状态和使用 fake runner 的编排。它们不能证明真实 MaaCore 的停止、回调时序和跨 session 复用行为。

The draft review should focus on ownership, API boundaries, and representative callers. The known runtime limitations are follow-up work before integration, rather than claims of supported behavior.

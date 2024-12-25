# maa-cli RPC API

## Current Status

Currently, maa-cli executes tasks through the following steps:

1. Parse MaaCore related configuration (profile/\*.toml);
2. Parse tasks: read task definition files for custom tasks, process command line arguments for predefined tasks;
3. Modify partial configuration according to tasks;
4. Load MaaCore and initialize according to configuration;
5. Add parsed tasks to MaaCore and start;
6. Wait for tasks to complete and exit program.

## Issues

The above implementation is simple and straightforward, but has the following issues:

- Loading and configuring MaaCore is required for each task execution, which takes some time (around a few seconds). This overhead can be ignored for longer tasks but becomes significant for simpler tasks, especially when executing multiple tasks consecutively via command line.
- Currently maa-cli has no external intervention methods after startup. Some MaaCore options allow runtime changes, but maa-cli's current implementation cannot support this.
- In the future, maa-cli can serve as a backend for other frontends to call. This way other frontends can avoid the relatively complex MaaCore FFI while also enabling WebUI development.

## Solution

Introduce a Server mode for maa-cli, started via `maa serve`. After startup, `maa-cli` will act as an RPC Server, listening on a Unix or TCP Socket. Commands like `maa run` and `maa startup` will parse tasks and send requests to the server. When running tasks, if the server is not started, maa-cli will start a server itself and keep it active as a daemon process in the background for a period of time.

## Implementation Details

- RPC Framework: JSON-RPC (crate jsonrpsee). Simple, easy to debug, natively supported by browsers, easy to integrate with existing code. RPC protocol related structs implemented in a separate crate.
- Transport: None or WebSocket or WS+TLS. WS is more efficient than HTTP, full-duplex, and like HTTP has native browser support.

## RPC API List

!!! Note
The following APIs are examples only, actual implementation may differ. JSON does not support comments, so comments in the JSON code below are for illustration only.

RPC API is implemented using JSON-RPC 2.0. All requests and responses are in JSON format. See [JSON-RPC 2.0](https://www.jsonrpc.org/specification) for the JSON-RPC 2.0 specification. The basic structure is as follows:

```json
{
  "jsonrpc": "2.0", // JSON-RPC version, must be "2.0"
  "method": "AppendTask", // Method name, must be string, see below for specific method names
  "params": {}, // Method parameters, optional, see below for specific parameters
  "id": 1 // Request ID, optional, used to identify request, server will return same ID in response
}
```

### MaaCore Task Related

#### Add Task

Add a task to MaaCore.

**Method Name**: `AsstAppendTask`

**Method Parameters**:

```jsonc
{
  "task_type": "StartUp", // Task type, see MAA integration docs for supported types
  "task_params": {}, // Task parameters, see MAA integration docs for details
  "process_task_params": false, // Whether to process task params, can be bool, string or string list. true processes all params, false processes none, string/string list processes specified param names
}
```

**Response Result**:

```json
{
  "task_id": 1 // Task ID, integer to identify task, greater than 0 and less than int32 max value (2^31 - 1)
}
```

### Modify Task Parameters

Modify parameters for a given task. MaaCore does not support removing tasks, but you can disable a task by setting its parameters to `{ "enabled": false }`.

**Method Name**: `AsstSetParams`

**Method Parameters**:

```json
{
  "task_id": 1, // Task ID returned by `AsstAppendTask`
  "task_params": {}, // New task parameters
  "process_task_params": false // Whether to process task parameters, e.g. convert relative paths to absolute paths, defaults to false
}
```

**Response Result**: None

### Start Tasks

**Method Name**: `AsstStartTasks`

**Method Parameters**: None

**Response Result**: None

### Stop Tasks

Stop executing tasks (all incomplete tasks will be stopped).

**Method Name**: `AsstStopTasks`

**Method Parameters**: None

**Response Result**: None

### Check if Tasks are Running

Check if any tasks are currently executing.

**Method Name**: `AsstIsRunning`

**Method Parameters**: None

**Response Result**:

```json
{
  "is_running": false // Whether tasks are currently executing
}
```

### Subscribe to Logs

Subscribe to maa-cli logs. After subscribing, maa-cli will push logs to the client.

**Method Name**: `SubscribeLog`

**Method Parameters**:

```json
{
  "level": "info" // Log level, see MAA integration docs for supported levels
}
```

## Server Control

### Get Alive Time

Get Server alive time. Server alive time is how long the Server will stay alive without any requests. Server will automatically close when alive time is reached.

**Method Name**: `GetAliveTime`

**Method Parameters**: None

**Response Result**:

```json
{
  "alive_time": 60 // Alive time in seconds
}
```

### Change Alive Time

**Method Name**: `SetAliveTime`

**Method Parameters**:

```json
{
  "alive_time": 60 // Alive time in seconds
}
```

**Response Result**: None

### Shutdown Server

Shutdown Server. If tasks are running, this method will return an error. Please stop all tasks before shutting down Server.

**Method Name**: `Terminate`

**Method Parameters**:

```json
{
  "wait_time": 60 // Wait time in seconds
}
```

**Response Result**:

```json
{
  "time": "2021-01-01T00:00:00Z" // Expected shutdown time
}
```

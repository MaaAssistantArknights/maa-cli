# Usage of `maa-cli`

`maa-cli` is a command line interface for `MaaCore` to automate Arknights game tasks. In addition, `maa-cli` can manage `MaaCore`.

## Manage MaaCore

`maa-cli` can install and update `MaaCore` and resources, just run the following commands:

```bash
maa install # Install MaaCore and resources
maa update # Update MaaCore and resources
```

## Update `maa-cli` itself

`maa-cli` can update itself, just run the following command:

```bash
maa self update
```

**Note**: Users who install `maa-cli` via package manager should use the package manager to update `maa-cli`, this command is invalid for these users.

## Run Tasks

The main feature of `maa-cli` is to run tasks, including predefined tasks and custom tasks.

### Predefined tasks

- `maa startup [client]`: start the game client and enter the main screen, the `client` is the client type of game, leave it empty to don't start the game;
- `maa closedown`: close the game client;
- `maa fight [stage]`: run a fight task, the `stage` is the stage to fight, like `1-7`, `CE-6`, etc.; if not given, it will be queried from user;
- `maa copilot <maa_uri>`: run a copilot task, the `maa_uri` is the URI of a copilot task; it can be `maa://1234` or local file path;
- `maa roguelike [theme]`: run a roguelike task, the `theme` is the theme of roguelike, available themes are `Phantom`, `Mizuki` and `Sami`.

### Custom Tasks

You can run custom tasks by `maa run <task>`. The `<task>` is the name of the custom task, which is defined in the configuration file. The location and format of the configuration file are described in [Custom Task Document][custom-task]. After defining the custom task, you can list all available tasks by `maa list`.

### Task Summary

`maa-cli` will output a summary of the task after the task is terminated, including the running time of each subtask (start time, end time, running time). For some tasks, it will also output a summary of the task results:

- `fight` task: stage name, times, sanity cost, and drop statistics;
- `infrast`: operators stationed in each facility, for the factory and trading post, it also includes the type of product;
- `recruit`: tags, star ratings, and status of each recruitment, as well as the total number of recruitments;
- `roguelike`: exploration times, investment times.

If you don't want the task summary, you can turn it off by `--no-summary`.

### Loggings

`maa-cli` will output logs, the log output levels from low to high are `Error`, `Warn`, `Info`, `Debug`, and `Trace`. The default log output level is `Warn`. The log level can be set by the `MAA_LOG` environment variable, for example, `MAA_LOG=debug`. You can also increase or decrease the log output level by `-v` or `-q`.

`maa-cli` will output logs to stderr by default. The `--log-file` option can output logs to a file, the logs are saved in `$(maa dir log)/YYYY/MM/DD/HH:MM:SS.log`, where `$(maa dir log)` is the log directory, you can get it by `maa dir log`. You can also specify the log file path by `--log-file=path/to/log`.

By default, all output logs will include a timestamp and a log level prefix. You can change this behavior by the `MAA_LOG_PREFIX` environment variable. When set to `Always`, the prefix will always be included, when set to `Auto`, the prefix will be included when writing to the log file, and not included when writing to stderr, and when set to `Never`, the prefix will not be included even when writing to the log file.

[custom-task]: config.md#custom-task

<!-- markdownlint-disable-file MD013 -->

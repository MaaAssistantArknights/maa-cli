# Fish completion for maa-cli

# Global options
complete -c maa -s v -l verbose -d 'Output more information, repeat to increase verbosity'
complete -c maa -s q -l quiet -d 'Output less information, repeat to increase quietness'
complete -c maa -l batch -d 'Enable touch mode'
complete -c maa -l log-file -d 'Log to file instead of stderr'

# help
set -l subcommands install update self hot-update dir version run fight copilot roguelike list complete
complete -c maa -s h -l help -d 'Print help (see more with \'--help\')'
complete -c maa -n "__fish_use_subcommand" -f -a "help" -d 'Print help for given subcommand'
complete -c maa -n "__fish_seen_subcommand_from help" -f -a "$subcommands"

# version
complete -c maa -n "__fish_use_subcommand" -s V -l version -d 'Print version'
complete -c maa -n "__fish_use_subcommand" -f -a "version" -d 'Print version of given component'
complete -c maa -n "__fish_seen_subcommand_from version" -f -a "all cli core"

# Subcommands
complete -c maa -n "__fish_use_subcommand" -f -a "install" -d 'Install maa maa_core and resources'
complete -c maa -n "__fish_use_subcommand" -f -a "update" -d 'Update maa maa_core and resources'
complete -c maa -n "__fish_use_subcommand" -f -a "self" -d 'Manage maa-cli self'
complete -c maa -n "__fish_use_subcommand" -f -a "hot-update" -d 'Hot update for resource'
complete -c maa -n "__fish_use_subcommand" -f -a "dir" -d 'Print path of maa directories'
complete -c maa -n "__fish_use_subcommand" -f -a "run" -d 'Run a predefined task'
complete -c maa -n "__fish_use_subcommand" -f -a "fight" -d 'Run fight task'
complete -c maa -n "__fish_use_subcommand" -f -a "copilot" -d 'Run copilot task'
complete -c maa -n "__fish_use_subcommand" -f -a "roguelike" -d 'Run rouge-like task'
complete -c maa -n "__fish_use_subcommand" -f -a "list" -d 'List all available tasks'
complete -c maa -n "__fish_use_subcommand" -f -a "complete" -d 'Generate completion script for given shell'

set -l channels alpha beta stable
# MaaCore installer options
complete -c maa -n "__fish_seen_subcommand_from install update" -f -a "$channels" -d 'Channel of MaaCore to install'
complete -c maa -n "__fish_seen_subcommand_from install update" -f -s t -l test-time -d 'Time to test download speed' -r
complete -c maa -n "__fish_seen_subcommand_from install update" -f -l api-url -d 'URL of api to get version information of MaaCore' -r
complete -c maa -n "__fish_seen_subcommand_from install update" -l no-resource -d 'Do not install resource of MaaCore'
complete -c maa -n "__fish_seen_subcommand_from install" -s f -l force -d 'Force to install even if the maa and resource already exists'

# MaaCLI self update options
complete -c maa -n "__fish_seen_subcommand_from self; and not __fish_seen_subcommand_from update" -f -a "update" -d 'Install maa-cli self'
complete -c maa -n "__fish_seen_subcommand_from self; and __fish_seen_subcommand_from update" -a "$channels" -d 'Channel of maa-cli to install'
complete -c maa -n "__fish_seen_subcommand_from self; and __fish_seen_subcommand_from update" -l api-url -d 'URL of api to get version information of maa-cli' -r
complete -c maa -n "__fish_seen_subcommand_from self; and __fish_seen_subcommand_from update" -l download-url -d 'URL of maa-cli to download' -r

# Maa directory navigation
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "data" -d "Print maa-cli's data directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "lib" -d "Print MaaCore library directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "config" -d "Print maa-cli's config directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "cache" -d "Print maa-cli's cache directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "resource" -d "Print MaaCore's resource directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "hot-update" -d "Print MaaCore's hot-update directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "log" -d "Print MaaCore's log directory"

# Maa run related options
## Common options
set -l run_commands run fight copilot roguelike
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -s a -l addr -d 'ADB serial number of device or MaaTools address set in PlayCover' -r
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -l user-resource -d 'Load resources from the config directory'
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -l dry-run -d 'Parse the your config but do not connect to the game'
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -l no-summary -d 'Do not print summary when finnish'
## command specific options
complete -c maa -n "__fish_seen_subcommand_from run" -f -a "$(maa list)"
complete -c maa -n "__fish_seen_subcommand_from fight" -l startup -d 'Whether to start the game'
complete -c maa -n "__fish_seen_subcommand_from fight" -l closedown -d 'Whether to close the game'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -a "phantom mizuki sami"

# Subcommand don't require arguments
complete -c maa -n "__fish_seen_subcommand_from hot-update list" -f # prevent fish complete from path

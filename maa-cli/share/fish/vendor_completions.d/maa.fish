# fish completion file for maa

# Top level
complete -c maa -n "__fish_use_subcommand" -s h -l help -d 'Print help'
complete -c maa -n "__fish_use_subcommand" -s V -l version -d 'Print version'
complete -c maa -n "__fish_use_subcommand" -f -a "install" -d 'Install maa core and resources'
complete -c maa -n "__fish_use_subcommand" -f -a "update" -d 'Update maa core and resources'
complete -c maa -n "__fish_use_subcommand" -f -a "self" -d 'Manage maa-cli self and maa-run'
complete -c maa -n "__fish_use_subcommand" -f -a "dir" -d 'Print path of maa directories'
complete -c maa -n "__fish_use_subcommand" -f -a "version" -d 'Print version of given component'
complete -c maa -n "__fish_use_subcommand" -f -a "run" -d 'Run a predefined task'
complete -c maa -n "__fish_use_subcommand" -f -a "list" -d 'List all available tasks'
complete -c maa -n "__fish_use_subcommand" -f -a "complete" -d 'Generate completion script for given shell'
complete -c maa -n "__fish_use_subcommand" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'

# Subcommands
## Install and Update
complete -c maa -n "__fish_seen_subcommand_from install update" -f -a "stable beta alpha" -d 'Install maa and resource from given channel'
complete -c maa -n "__fish_seen_subcommand_from install update" -s t -l test-time -d 'Time to test download speed' -r
complete -c maa -n "__fish_seen_subcommand_from install update" -l no-resource -d 'Do not install resource'
complete -c maa -n "__fish_seen_subcommand_from install update" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c maa -n "__fish_seen_subcommand_from install" -s f -l force -d 'Force to install even if the maa and resource already exists'
## Self
complete -c maa -n "__fish_seen_subcommand_from self" -f -a "update" -d 'Update maa-cli self'
complete -c maa -n "__fish_seen_subcommand_from self" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c maa -n "__fish_seen_subcommand_from self" -s h -l help -d 'Print help (see more with \'--help\')'
## Dir
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "data" -d "Print maa-cli's data directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "lib" -d "Print MaaCore library directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "config" -d "Print maa-cli's config directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "cache" -d "Print maa-cli's cache directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "resource" -d "Print MaaCore's resource directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "log" -d "Print MaaCore's log directory"
complete -c maa -n "__fish_seen_subcommand_from dir" -s h -l help -d 'Print help (see more with \'--help\')'
## Version
complete -c maa -n "__fish_seen_subcommand_from version" -f -a "all maa-cli maa-core" -d 'Print version of given component'
complete -c maa -n "__fish_seen_subcommand_from version" -s h -l help -d 'Print help (see more with \'--help\')'
## Run
complete -c maa -n "__fish_seen_subcommand_from run" -f -a "$(maa list)"
complete -c maa -n "__fish_seen_subcommand_from run" -s a -l addr -d 'ADB serial number of device or MaaTools address set in PlayCover' -r
complete -c maa -n "__fish_seen_subcommand_from run" -s v -l verbose -d 'Output more information, repeat to increase verbosity'
complete -c maa -n "__fish_seen_subcommand_from run" -s q -l quiet -d 'Output less information, repeat to increase quietness'
complete -c maa -n "__fish_seen_subcommand_from run" -l user-resource -d 'Load resources from the user config directory'
complete -c maa -n "__fish_seen_subcommand_from run" -s h -l help -d 'Print help (see more with \'--help\')'
## Help
complete -c maa -n "__fish_seen_subcommand_from help" -f -a "install update self dir version run list complete" -d 'Print help of given subcommand(s)'

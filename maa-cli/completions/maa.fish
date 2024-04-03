# Fish completion for maa-cli

# Global options
complete -c maa -s v -l verbose -d 'Output more information, repeat to increase verbosity'
complete -c maa -s q -l quiet -d 'Output less information, repeat to increase quietness'
complete -c maa -l batch -d 'Enable touch mode'
complete -c maa -l log-file -d 'Log to file instead of stderr'

# Subcommands
set -g __maa_subcommands

function __maa_add_subcommand
    set -l subcommand $argv[1]
    set -l description $argv[2]
    set -a __maa_subcommands $subcommand
    complete -c maa -n __fish_use_subcommand -f -a $subcommand -d $description
end

__maa_add_subcommand help 'Print help for given subcommand'
__maa_add_subcommand version 'Print version of given component'
__maa_add_subcommand install 'Install maa maa_core and resources'
__maa_add_subcommand update 'Update maa maa_core and resources'
__maa_add_subcommand self 'Manage maa-cli self'
__maa_add_subcommand hot-update 'Hot update for resource'
__maa_add_subcommand dir 'Print path of maa directories'
__maa_add_subcommand run 'Run a predefined task'
__maa_add_subcommand startup 'Start game and enter main screen'
__maa_add_subcommand closedown 'Close game'
__maa_add_subcommand fight 'Run fight task'
__maa_add_subcommand copilot 'Run copilot task'
__maa_add_subcommand roguelike 'Run rogue-like task'
__maa_add_subcommand activity 'Show stage activity of given client'
__maa_add_subcommand remainder 'Get remainder of given divisor and current date'
__maa_add_subcommand list 'List all available tasks'
__maa_add_subcommand complete 'Generate completion script for given shell'
__maa_add_subcommand mangen 'Generate man page for maa-cli at given path'
__maa_add_subcommand convert 'Convert config file to another format'
__maa_add_subcommand cleanup 'Cleanup maa-cli and MaaCore cache'

# options for subcommands
# help
complete -c maa -n "__fish_seen_subcommand_from help" -f -a "$__maa_subcommands"
complete -c maa -s h -l help -d 'Print help (see more with \'--help\')'

# version
complete -c maa -n "__fish_seen_subcommand_from version" -f -a "all maa-core maa-cli" -d 'Component to print version'
complete -c maa -n __fish_use_subcommand -s V -l version -d 'Print version'

# Install and update options
set -l channels alpha beta stable
## MaaCore installer options
complete -c maa -n "__fish_seen_subcommand_from install update" -f -a "$channels" -d 'Channel of MaaCore to install'
complete -c maa -n "__fish_seen_subcommand_from install update" -f -s t -l test-time -d 'Time to test download speed' -r
complete -c maa -n "__fish_seen_subcommand_from install update" -f -l api-url -d 'URL of api to get version information of MaaCore' -r
complete -c maa -n "__fish_seen_subcommand_from install update" -l no-resource -d 'Do not install resource of MaaCore'
complete -c maa -n "__fish_seen_subcommand_from install" -s f -l force -d 'Force to install even if the maa and resource already exists'
## MaaCLI self update options
complete -c maa -n "__fish_seen_subcommand_from self; and not __fish_seen_subcommand_from update" -f -a update -d 'Install maa-cli self'
complete -c maa -n "__fish_seen_subcommand_from self; and __fish_seen_subcommand_from update" -a "$channels" -d 'Channel of maa-cli to install'
complete -c maa -n "__fish_seen_subcommand_from self; and __fish_seen_subcommand_from update" -f -l api-url -d 'URL of api to get version information of maa-cli' -r
complete -c maa -n "__fish_seen_subcommand_from self; and __fish_seen_subcommand_from update" -l download-url -d 'URL of maa-cli to download' -r

# Maa directory navigation
set -l maa_dirs data lib config cache resource hot-update log
complete -c maa -n "__fish_seen_subcommand_from dir" -f -a "$maa_dirs"

# Maa run related options
set -l clients Official Bilibili Txwy YoStarEN YoStarJP YoStarKR
## Common options
set -l run_commands run startup closedown fight copilot roguelike
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -s a -l addr -d 'ADB serial number of device or MaaTools address set in PlayCover' -r
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -s p -l profile -d 'Profile to use' -r
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -l user-resource -d 'Load resources from the config directory'
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -l dry-run -d 'Parse the your config but do not connect to the game'
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -l no-summary -d 'Do not print summary when finnish'
complete -c maa -n "__fish_seen_subcommand_from $run_commands" -f # prevent fish complete from path
## command specific options
complete -c maa -n "__fish_seen_subcommand_from run" -f -a "$(maa list)"
complete -c maa -n "__fish_seen_subcommand_from startup" -f -a "$clients"
complete -c maa -n "__fish_seen_subcommand_from startup" -f -l account -d 'Account to login' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -s m -l medicine -d 'Medicine to use' -r
complete -c maa -n "__fish_seen_subcommand_from roguelike" -a "phantom mizuki sami"

# Misc commands
complete -c maa -n "__fish_seen_subcommand_from complete" -f -a "bash fish zsh powershell"
complete -c maa -n "__fish_seen_subcommand_from mangen" -l path -r -d 'Directory to save man pages'
complete -c maa -n "__fish_seen_subcommand_from cleanup" -f -a "cli-cache avatars log misc"
complete -c maa -n "__fish_seen_subcommand_from convert" -f -s f -l format -a "j json y yaml t toml" -r
complete -c maa -n "__fish_seen_subcommand_from activity" -f -a "$clients"
complete -c maa -n "__fish_seen_subcommand_from remainder" -f -l timezone -d 'Timezone to determine the current date' -r
complete -c maa -n "__fish_seen_subcommand_from hot-update list" -f # prevent fish complete from path

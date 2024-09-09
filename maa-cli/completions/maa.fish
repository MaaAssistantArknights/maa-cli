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
__maa_add_subcommand import 'Import config file from file'
__maa_add_subcommand init 'Initialize config file'
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
set -l run_commands run startup closedown fight copilot sscopilot roguelike reclamation
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
complete -c maa -n "__fish_seen_subcommand_from closedown" -f -a "$clients"

complete -c maa -n "__fish_seen_subcommand_from fight" -f -s m -l medicine -d 'Medicine to use' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l expiring-medicine -d 'Expiring medicine to use' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l stone -d 'Stone to use' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l times -d 'Times to fight' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -s D -l drops -d 'Exit after collecting given number of drops' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l series -d 'Series of proxy combat' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l report-to-penguin -d 'Report drops to the Penguin Statistics'
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l penguin-id -d 'Penguin Statistics ID to report drops' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l report-to-yituliu -d 'Report drops to the yituliu'
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l yituliu-id -d 'Yituliu ID to report drops' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l client-type -d 'Client type to restart' -r
complete -c maa -n "__fish_seen_subcommand_from fight" -f -l dr-grandet -d 'Use Dr. Grandet'

complete -c maa -n "__fish_seen_subcommand_from copilot" -f -s f -l formation -d 'Use formation'
complete -c maa -n "__fish_seen_subcommand_from copilot" -f -l use-sanity-potion -d 'Use sanity potion'
complete -c maa -n "__fish_seen_subcommand_from copilot" -f -l need-navigate -d 'Need navigate'
complete -c maa -n "__fish_seen_subcommand_from copilot" -f -l add-trust -d 'Add trust'
complete -c maa -n "__fish_seen_subcommand_from copilot" -f -l select-formation -d 'Select formation' -r
complete -c maa -n "__fish_seen_subcommand_from copilot" -f -l support-unit-name -d 'Support unit name' -r
complete -c maa -n "__fish_seen_subcommand_from sscopilot" -f -s l -l loop-times -d 'Loop times' -r

complete -c maa -n "__fish_seen_subcommand_from roguelike" -a "Phantom Mizuki Sami Sarkaz"
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l mode -d 'Mode of roguelike' -a "0 1 2 3 4"
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l squad -d 'Squad to use' -r
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l roles -d 'Roles to use' -r
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l core-char -d 'Core character to use' -r
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l start-count -d 'Count of start' -r
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l disable-investment -d 'Disable investment'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l investment-with-more-score -d 'Try to gain more score in investment mode'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l investments-count -d 'Count of investments' -r
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l no-stop-when-investment-full -d 'Do not stop when investment is full'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l use-support -d 'Use support'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l use-nonfriend-support -d 'Use non-friend support'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l start-with-elite-two -d 'Start with elite two'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l only-start-with-elite-two -d 'Only start with elite two'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l stop-at-final-boss -d 'Stop exploration before final boss'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l refresh-trader-with-dice -d 'Refresh trader with dice'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l use-foldartal -d 'Use foldartal'
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l start-foldartals -d 'Start foldartals' -r
complete -c maa -n "__fish_seen_subcommand_from roguelike" -l expected-collapsal-paradigms -d 'Expected collapsal paradigms' -r

complete -c maa -n "__fish_seen_subcommand_from reclamation" -a "Tales"
complete -c maa -n "__fish_seen_subcommand_from reclamation" -f -s m -l mode -d 'Mode of reclamation' -r -a "0 1"
complete -c maa -n "__fish_seen_subcommand_from reclamation" -f -s C -l tool-to-craft -d 'Tool to craft' -r
complete -c maa -n "__fish_seen_subcommand_from reclamation" -f -s m -l increase-mode -d 'Method to increase the number of tools' -r
complete -c maa -n "__fish_seen_subcommand_from reclamation" -f -s n -l num-craft-batches -d 'Number of batches in each game run' -r

# Misc commands
complete -c maa -n "__fish_seen_subcommand_from complete" -f -a "bash fish zsh powershell"
complete -c maa -n "__fish_seen_subcommand_from mangen" -l path -r -d 'Directory to save man pages'
complete -c maa -n "__fish_seen_subcommand_from cleanup" -f -a "cli-cache core-cache debug log"
complete -c maa -n "__fish_seen_subcommand_from convert" -f -s f -l format -a "j json y yaml t toml" -r
complete -c maa -n "__fish_seen_subcommand_from activity" -f -a "$clients"
complete -c maa -n "__fish_seen_subcommand_from remainder" -f -l timezone -d 'Timezone to determine the current date' -r

complete -c maa -n "__fish_seen_subcommand_from import" -s f -l force -d 'Force to import even if the config already exists'
complete -c maa -n "__fish_seen_subcommand_from import" -s t -l config-type -d "Type of config file to import, default to task"

complete -c maa -n "__fish_seen_subcommand_from init" -s n -l name -d 'Name of profile to initialize' -r
complete -c maa -n "__fish_seen_subcommand_from init" -l force -d 'Force to initialize even if the config already exists'
complete -c maac -n "__fish_seen_subcommand_from init" -s f -l format -a "j json y yaml t toml" -r

complete -c maa -n "__fish_seen_subcommand_from hot-update list" -f # prevent fish complete from path

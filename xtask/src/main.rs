use anyhow::Result;

mod cmd;
mod env;
mod github;
mod release;

fn print_help() {
    println!("xtask - Task automation for maa-cli");
    println!();
    println!("USAGE:");
    println!("    xtask <COMMAND>");
    println!();
    println!("COMMANDS:");
    println!("    release    Release automation tasks");
    println!("    help       Print this message");
    println!();
    println!("Run 'xtask <COMMAND> help' for more information on a command.");
}

fn print_release_help() {
    println!("Release automation tasks");
    println!();
    println!("USAGE:");
    println!("    xtask release <COMMAND>");
    println!();
    println!("COMMANDS:");
    println!("    meta       Parse version and determine release metadata");
    println!("    package    Update version.json files with release information");
    println!("    help       Print this message");
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);

    let command = match args.next() {
        Some(cmd) => cmd,
        None => {
            print_help();
            std::process::exit(1);
        }
    };

    match command.as_str() {
        "release" => {
            let subcommand = match args.next() {
                Some(cmd) => cmd,
                None => {
                    print_release_help();
                    std::process::exit(1);
                }
            };

            match subcommand.as_str() {
                "meta" => release::meta::run(),
                "package" => release::package::run(),
                "help" | "-h" | "--help" => {
                    print_release_help();
                    Ok(())
                }
                _ => {
                    eprintln!("error: unrecognized subcommand '{}'", subcommand);
                    eprintln!();
                    print_release_help();
                    std::process::exit(1);
                }
            }
        }
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("error: unrecognized command '{}'", command);
            eprintln!();
            print_help();
            std::process::exit(1);
        }
    }
}

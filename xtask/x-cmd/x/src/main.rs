//! 将 `cargo x <cmd>` 转发到 `x-<cmd>`，并折叠 CI 中的工具编译日志。

use std::{
    ffi::OsString,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let mut command = args.next();
    if command.as_deref() == Some(std::ffi::OsStr::new("--")) {
        command = args.next();
    }
    let Some(command) = command else {
        eprintln!("Usage: cargo x <command> [args...]");
        return ExitCode::FAILURE;
    };
    let mut package = OsString::from("x-");
    package.push(command);

    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        eprintln!("::group::Prepare {}", package.display());
    }
    let mut cargo = Command::new("cargo");
    cargo.args(["run", "-p"]).arg(package).arg("--").args(args);

    #[cfg(unix)]
    let error = {
        use std::os::unix::process::CommandExt;
        cargo.exec()
    };
    #[cfg(not(unix))]
    let error = match cargo.status() {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(error) => error,
    };

    eprintln!("Failed to run cargo: {error}");
    ExitCode::FAILURE
}

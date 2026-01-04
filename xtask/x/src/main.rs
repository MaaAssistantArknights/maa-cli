use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    println!("::group::Prepare xtask");

    let status = Command::new("cargo")
        .args(["run", "-pxtask"])
        .args(std::env::args().skip(1))
        .status()
        .expect("failed to run cargo");

    match status.code() {
        Some(code) => ExitCode::from(code as u8),
        None => ExitCode::FAILURE,
    }
}

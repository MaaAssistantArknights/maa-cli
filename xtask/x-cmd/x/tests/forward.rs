use std::process::Command;

#[test]
fn missing_command_reports_usage() -> std::io::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_x")).output()?;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"Usage: cargo x <command> [args...]\n");
    Ok(())
}

#[cfg(unix)]
#[test]
fn forwarding_preserves_arguments_exit_status_and_stdout() -> std::io::Result<()> {
    use std::{fs, os::unix::fs::PermissionsExt};

    let temp = tempfile::tempdir()?;
    let cargo = temp.path().join("cargo");
    fs::write(&cargo, "#!/bin/sh\nprintf '%s\\n' \"$@\"\nexit 37\n")?;
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755))?;
    for (args, expected_stdout, expected_stderr) in [
        (
            vec!["channel", "--", "argument with spaces"],
            "run\n-p\nx-channel\n--\n--\nargument with spaces\n",
            "::group::Prepare x-channel\n",
        ),
        (
            vec!["--", "channel", "--", "argument with spaces"],
            "run\n-p\nx-channel\n--\n--\nargument with spaces\n",
            "::group::Prepare x-channel\n",
        ),
        (
            vec!["package", "argument with spaces"],
            "run\n-p\nx-package\n--release\n--\nargument with spaces\n",
            "::group::Prepare x-package\n",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_x"))
            .args(args)
            .env("PATH", temp.path())
            .env("GITHUB_ACTIONS", "true")
            .output()?;
        assert_eq!(output.status.code(), Some(37));
        assert_eq!(output.stdout, expected_stdout.as_bytes());
        assert_eq!(output.stderr, expected_stderr.as_bytes());
    }
    Ok(())
}

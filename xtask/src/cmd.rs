//! Simple command execution helpers.

use std::process::Command;

use anyhow::{Context, Result, bail};

pub trait CommandExt {
    fn env_vars(&mut self, env_vars: &EnvVars);

    fn run(&mut self) -> Result<()>;
}

impl CommandExt for Command {
    fn env_vars(&mut self, env_vars: &EnvVars) {
        for (key, value) in env_vars.0.iter() {
            self.env(key, value);
        }
    }

    fn run(&mut self) -> Result<()> {
        println!("Running command: {:?}", self);
        let status = self.status().context("Failed to execute command")?;
        if !status.success() {
            bail!("Failed to run: {self:?}");
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct EnvVars<'s>(Vec<(&'s str, String)>);

impl<'s> EnvVars<'s> {
    pub fn new() -> Self {
        EnvVars(Vec::new())
    }

    pub fn push(&mut self, key: &'s str, value: String) {
        println!("{key}={value}");
        self.0.push((key, value));
    }
}

pub fn cargo() -> Command {
    Command::new("cargo")
}

pub fn rustup_up(channel: &str) -> Command {
    let mut cmd = Command::new("rustup");
    cmd.args(["install", channel, "--no-self-update"]);
    cmd
}

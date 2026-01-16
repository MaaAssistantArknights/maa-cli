//! Grouped logging helpers that work on GitHub Actions and locally.

use std::sync::LazyLock;

pub fn is_github_actions() -> bool {
    std::env::var_os("GITHUB_ACTIONS").is_some()
}

pub static GROUP_START_PREFIX: LazyLock<&'static str> = LazyLock::new(|| {
    if is_github_actions() {
        "::group::"
    } else {
        "==> "
    }
});
pub static GROUP_END_LINE: LazyLock<&'static str> = LazyLock::new(|| {
    if is_github_actions() {
        "::endgroup::"
    } else {
        ""
    }
});

pub struct Group<'s> {
    name: &'s str,
}

impl<'s> Group<'s> {
    pub fn new(name: &'s str) -> Self {
        Self { name }
    }

    pub fn start(&self) {
        println!("{}{}", *GROUP_START_PREFIX, self.name);
    }

    pub fn end(&self) {
        if !GROUP_END_LINE.is_empty() {
            println!("{}", *GROUP_END_LINE);
        }
    }

    pub fn run<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce() -> anyhow::Result<()>,
    {
        self.start();
        f()?;
        self.end();

        Ok(())
    }
}

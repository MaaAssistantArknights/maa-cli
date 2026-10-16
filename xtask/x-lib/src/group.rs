//! Grouped logging helpers that work on GitHub Actions and locally.

use std::sync::LazyLock;

pub fn is_github_actions() -> bool {
    std::env::var_os("GITHUB_ACTIONS").is_some()
}

static GROUP_START_PREFIX: LazyLock<&'static str> = LazyLock::new(|| {
    if is_github_actions() {
        "::group::"
    } else {
        "==> "
    }
});
static GROUP_END_LINE: LazyLock<&'static str> = LazyLock::new(|| {
    if is_github_actions() {
        "::endgroup::"
    } else {
        ""
    }
});

pub fn start(name: &str) {
    eprintln!("{}{name}", *GROUP_START_PREFIX);
}

pub fn end() {
    if !GROUP_END_LINE.is_empty() {
        eprintln!("{}", *GROUP_END_LINE);
    }
}

/// Close the compilation group opened by `cargo x` before running the command.
pub fn finish_prepare() {
    end();
}

pub struct Group<'s> {
    name: &'s str,
}

impl<'s> Group<'s> {
    pub fn new(name: &'s str) -> Self {
        Self { name }
    }

    pub fn start(&self) {
        start(self.name);
    }

    pub fn end(&self) {
        end();
    }

    pub fn run<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.start();
        let result = f();
        self.end();

        result
    }
}

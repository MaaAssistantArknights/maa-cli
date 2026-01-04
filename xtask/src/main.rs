use anyhow::Result;
use clap::{Parser, Subcommand};

mod cmd;
mod env;
mod github;
mod release;
mod test;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Task automation for maa-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Release automation tasks
    Release {
        #[command(subcommand)]
        command: release::ReleaseCommands,
    },
    /// Run tests with optional core installation and coverage
    Test(TestOptions),
}

#[derive(Parser)]
struct TestOptions {
    /// Install MaaCore before testing (if false, sets SKIP_CORE_TEST=true)
    #[arg(long)]
    with_core: bool,

    /// Enable coverage
    #[arg(long, default_value = "None")]
    coverage: CoverageMode,

    /// Disable cargo clippy
    #[arg(long)]
    no_clippy: bool,

    /// Disable all features
    #[arg(long)]
    no_all_features: bool,

    /// Package name to test, `workspace` means `--workspace`
    #[arg(short, long, default_value = "workspace")]
    package: String,

    /// Additional arguments to pass to `cargo test`
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    test_args: Vec<String>,
}

impl TestOptions {
    fn package_flags(&self) -> Vec<&str> {
        match self.package.as_str() {
            "workspace" => vec!["--workspace"],
            _ => vec!["--package", &self.package],
        }
    }
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum CoverageMode {
    TestOnly,
    All,
    None,
}

impl CoverageMode {
    fn coverage_run(self) -> bool {
        matches!(self, Self::All)
    }

    fn coverage_test(self) -> bool {
        !matches!(self, Self::None)
    }

    fn report(self) -> bool {
        !matches!(self, Self::None)
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Release { command } => release::run(command),
        Commands::Test(options) => test::run_tests(options),
    }
}

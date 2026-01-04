use anyhow::Result;
use clap::{Parser, Subcommand};

mod build;
mod cmd;
mod env;
mod github;
mod release;
mod test;

const CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");
const HOST_TRIPLET: &str = env!("TARGET");

fn workspace_root() -> &'static str {
    std::path::Path::new(CARGO_MANIFEST_DIR)
        .parent()
        .expect("CARGO_MANIFEST_DIR should have a parent directory")
        .to_str()
        .expect("workspace rot path should be valid UTF-8")
}

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Task automation for maa-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build maa-cli binary
    Build(BuildOptions),
    /// Release automation tasks
    Release {
        #[command(subcommand)]
        command: release::ReleaseCommands,
    },
    /// Run tests with optional core installation and coverage
    Test(TestOptions),
}

#[derive(Parser)]
struct BuildOptions {
    /// Build profile to use
    #[arg(long, default_value = "dev")]
    pub profile: String,

    /// Use vendored dependencies instead of system dependencies
    #[arg(long)]
    pub vendored_deps: bool,

    /// Create tar package after build
    #[arg(long)]
    pub tar: bool,

    /// Additional arguments to pass to `cargo build`
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub build_args: Vec<String>,
}

#[derive(Parser)]
struct TestOptions {
    /// Install MaaCore before testing (if false, sets SKIP_CORE_TEST=true)
    #[arg(long)]
    with_core: bool,

    /// Enable coverage
    #[arg(long, default_value = "none")]
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
            "workspace" => vec!["--workspace", "--exclude", "xtask", "--exclude", "x"],
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
    println!("::endgroup::");

    let cli = Cli::parse();

    match cli.command {
        Commands::Build(options) => build::run(options),
        Commands::Release { command } => release::run(command),
        Commands::Test(options) => test::run_tests(options),
    }
}

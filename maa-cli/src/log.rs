use std::path::PathBuf;

#[derive(clap::Args)]
pub struct Args {
    #[arg(
        short = 'v',
        long,
        action = clap::ArgAction::Count,
        global = true,
        help = fl!("verbose-help"),
    )]
    verbose: u8,
    #[arg(
        short = 'q',
        long,
        action = clap::ArgAction::Count,
        global = true,
        help = fl!("quiet-help"),
    )]
    quiet: u8,
    #[arg(long, global = true, require_equals = true, value_name = "PATH",
          help = fl!("log-file-help"), long_help = fl!("log-file-long-help"))]
    log_file: Option<Option<std::path::PathBuf>>,
}

impl Args {
    fn log_level(&self) -> u8 {
        let default_level = std::env::var_os("MAA_LOG")
            .and_then(|s| s.to_str().and_then(|s| s.parse().ok()))
            .unwrap_or(log::Level::Warn);

        (default_level as u8 + self.verbose).saturating_sub(self.quiet)
    }

    pub fn to_filter(&self) -> log::LevelFilter {
        use log::LevelFilter::*;
        match self.log_level() {
            0 => Off,
            1 => Error,
            2 => Warn,
            3 => Info,
            4 => Debug,
            _ => Trace,
        }
    }

    // Accessors only used in tests
    // this function consumes entire self, while in init_logger()
    // we need to use self.log_file, which only consumes part of self
    #[cfg(test)]
    pub fn log_file(self) -> Option<PathBuf> {
        log_path(self.log_file)
    }

    pub fn init_logger(self) -> anyhow::Result<()> {
        let mut builder = env_logger::Builder::new();

        builder.filter_level(self.to_filter());
        builder.format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "[{} {:<5}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                buf.default_styled_level(record.level()),
                record.args()
            )
        });

        if let Some(path) = log_path(self.log_file) {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            builder.target(env_logger::Target::Pipe(Box::new(file)));
        }

        builder.init();

        Ok(())
    }
}

fn log_path(path: Option<Option<PathBuf>>) -> Option<PathBuf> {
    use crate::dirs::{self, Ensure};
    path.map(|path| {
        path.unwrap_or_else(|| {
            let now = chrono::Local::now();
            let dir = dirs::log()
                .join(now.format("%Y").to_string())
                .join(now.format("%m").to_string())
                .join(now.format("%d").to_string());

            dir.ensure().unwrap();

            dir.join(format!("{}.log", now.format("%H:%M:%S")))
        })
    })
}

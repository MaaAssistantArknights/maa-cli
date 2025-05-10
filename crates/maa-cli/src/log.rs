use std::{io::Write, path::PathBuf};

#[derive(clap::Args)]
pub struct Args {
    #[arg(
        short = 'v',
        long,
        action = clap::ArgAction::Count,
        global = true,
    )]
    /// Increase verbosity, repeat for more verbosity
    verbose: u8,
    #[arg(
        short = 'q',
        long,
        action = clap::ArgAction::Count,
        global = true,
    )]
    /// Decrease verbosity, repeat for more quiet
    quiet: u8,
    /// Redirect log to file instead of stderr
    ///
    /// If no log file is specified, the log will be written to
    /// `$(maa dir log)/YYYY/MM/DD/HH:MM:SS.log`.
    #[arg(long, global = true, require_equals = true, value_name = "PATH")]
    log_file: Option<Option<PathBuf>>,
}

impl Args {
    fn log_level(&self) -> u8 {
        let default_level = std::env::var_os("MAA_LOG")
            .and_then(|s| s.to_str().and_then(|s| s.parse().ok()))
            .unwrap_or(log::Level::Warn);

        (default_level as u8 + self.verbose).saturating_sub(self.quiet)
    }

    fn to_filter(&self) -> log::LevelFilter {
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
    fn log_file(self) -> Option<PathBuf> {
        log_path(self.log_file)
    }

    pub fn init_logger(self) -> anyhow::Result<()> {
        let mut builder = env_logger::Builder::new();

        builder.filter_level(self.to_filter());
        builder.format(LogPrefix::from_env().format(self.log_file.is_some()));

        if let Some(path) = log_path(self.log_file) {
            if let Some(dir) = path.parent() {
                use crate::dirs::Ensure;
                dir.ensure()?;
            }
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
    path.map(|path| {
        path.unwrap_or_else(|| {
            let now = chrono::Local::now();
            let dir = crate::dirs::log()
                .join(now.format("%Y").to_string())
                .join(now.format("%m").to_string())
                .join(now.format("%d").to_string());

            dir.join(format!("{}.log", now.format("%H:%M:%S")))
        })
    })
}

/// Whether or not to print log prefix [YYYY-MM-DD HH:MM:SS LEVEL]
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Clone, Copy, Default)]
enum LogPrefix {
    /// Print log prefix if log to file, not print log prefix if log to stderr
    Auto,
    /// Always print log prefix
    #[default]
    Always,
    /// Never print log prefix
    Never,
}

impl LogPrefix {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "ALWAYS" | "Always" | "always" => Some(LogPrefix::Always),
            "NEVER" | "Never" | "never" => Some(LogPrefix::Never),
            "AUTO" | "Auto" | "auto" => Some(LogPrefix::Auto),
            _ => None,
        }
    }

    fn from_env() -> Self {
        std::env::var_os("MAA_LOG_PREFIX")
            .and_then(|s| s.to_str().and_then(LogPrefix::from_str))
            .unwrap_or_default()
    }

    fn format(
        &self,
        log_file: bool,
    ) -> fn(&mut env_logger::fmt::Formatter, &log::Record) -> std::io::Result<()> {
        match self {
            LogPrefix::Always => prefixed_format,
            LogPrefix::Never => plain_format,
            LogPrefix::Auto => {
                if log_file {
                    prefixed_format
                } else {
                    plain_format
                }
            }
        }
    }
}

fn prefixed_format(
    buf: &mut env_logger::fmt::Formatter,
    record: &log::Record,
) -> std::io::Result<()> {
    writeln!(
        buf,
        "[{} {}{:<5}{}] {}",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        buf.default_level_style(record.level()),
        record.level(),
        env_logger::fmt::style::Reset,
        record.args()
    )
}

fn plain_format(buf: &mut env_logger::fmt::Formatter, record: &log::Record) -> std::io::Result<()> {
    writeln!(buf, "{}", record.args())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    mod cmd_args {
        use std::env;

        use crate::command::parse_from;

        #[test]
        fn to_filter() {
            // Safety: MAA_LOG is only modify and read in this test
            unsafe { std::env::remove_var("MAA_LOG") };

            use log::LevelFilter::*;
            assert_eq!(parse_from(["maa", "list"]).log.to_filter(), Warn);
            assert_eq!(parse_from(["maa", "-v", "list"]).log.to_filter(), Info);
            assert_eq!(parse_from(["maa", "list", "-v"]).log.to_filter(), Info);

            assert_eq!(
                parse_from(["maa", "list", "--verbose"]).log.to_filter(),
                Info
            );
            assert_eq!(
                parse_from(["maa", "--verbose", "list"]).log.to_filter(),
                Info
            );
            assert_eq!(
                parse_from(["maa", "list", "--quiet"]).log.to_filter(),
                Error
            );

            assert_eq!(parse_from(["maa", "list", "-vvvv"]).log.to_filter(), Trace);
            assert_eq!(parse_from(["maa", "list", "-vvv"]).log.to_filter(), Trace);
            assert_eq!(parse_from(["maa", "list", "-vv"]).log.to_filter(), Debug);
            assert_eq!(parse_from(["maa", "list", "-v"]).log.to_filter(), Info);
            assert_eq!(parse_from(["maa", "list"]).log.to_filter(), Warn);
            assert_eq!(parse_from(["maa", "list", "-vq"]).log.to_filter(), Warn);
            assert_eq!(parse_from(["maa", "list", "-q"]).log.to_filter(), Error);
            assert_eq!(parse_from(["maa", "list", "-qq"]).log.to_filter(), Off);
            assert_eq!(parse_from(["maa", "list", "-qqq"]).log.to_filter(), Off);

            assert_eq!(parse_from(["maa", "list", "-vv"]).log.to_filter(), Debug);

            assert_eq!(parse_from(["maa", "list", "-q"]).log.to_filter(), Error);

            unsafe {
                env::set_var("MAA_LOG", "Info");
                assert_eq!(parse_from(["maa", "list"]).log.to_filter(), Info);
                env::set_var("MAA_LOG", "Debug");
                assert_eq!(parse_from(["maa", "list"]).log.to_filter(), Debug);
                env::set_var("MAA_LOG", "Trace");
                assert_eq!(parse_from(["maa", "list"]).log.to_filter(), Trace);
                env::remove_var("MAA_LOG");
            }
        }

        #[test]
        fn log_path() {
            use std::path::Path;
            assert!(parse_from(["maa", "list"]).log.log_file().is_none());
            assert!(
                parse_from(["maa", "list", "--log-file"])
                    .log
                    .log_file()
                    .is_some_and(|x| {
                        let now = chrono::Local::now();
                        let dir = crate::dirs::log()
                            .join(now.format("%Y").to_string())
                            .join(now.format("%m").to_string())
                            .join(now.format("%d").to_string());

                        // the file name is dependent on the current time, it's hard to test
                        x.starts_with(dir)
                    })
            );
            assert!(
                parse_from(["maa", "list", "--log-file=path"])
                    .log
                    .log_file()
                    .is_some_and(|x| x == Path::new("path"))
            );
        }
    }

    mod log_prefix {
        use super::*;

        #[test]
        fn from_str() {
            assert_eq!(LogPrefix::from_str("Always"), Some(LogPrefix::Always));
            assert_eq!(LogPrefix::from_str("always"), Some(LogPrefix::Always));
            assert_eq!(LogPrefix::from_str("NEVER"), Some(LogPrefix::Never));
            assert_eq!(LogPrefix::from_str("never"), Some(LogPrefix::Never));
            assert_eq!(LogPrefix::from_str("AUTO"), Some(LogPrefix::Auto));
            assert_eq!(LogPrefix::from_str("auto"), Some(LogPrefix::Auto));
            assert_eq!(LogPrefix::from_str("unknown"), None);
        }

        #[test]
        fn from_env() {
            // Safety: MAA_LOG_PREFIX only modify and read in this test
            unsafe {
                std::env::remove_var("MAA_LOG_PREFIX");
                assert_eq!(LogPrefix::from_env(), LogPrefix::Always);

                std::env::set_var("MAA_LOG_PREFIX", "Always");
                assert_eq!(LogPrefix::from_env(), LogPrefix::Always);

                std::env::set_var("MAA_LOG_PREFIX", "Never");
                assert_eq!(LogPrefix::from_env(), LogPrefix::Never);

                std::env::set_var("MAA_LOG_PREFIX", "Auto");
                assert_eq!(LogPrefix::from_env(), LogPrefix::Auto);

                std::env::set_var("MAA_LOG_PREFIX", "unknown");
                assert_eq!(LogPrefix::from_env(), LogPrefix::Always);
            }
        }

        #[test]
        fn format() {
            let pff = prefixed_format
                as fn(&mut env_logger::fmt::Formatter, &log::Record) -> std::io::Result<()>;
            let plf = plain_format
                as fn(&mut env_logger::fmt::Formatter, &log::Record) -> std::io::Result<()>;

            assert_eq!(LogPrefix::Always.format(true), pff);
            assert_eq!(LogPrefix::Always.format(false), pff);

            assert_eq!(LogPrefix::Never.format(true), plf);
            assert_eq!(LogPrefix::Never.format(false), plf);

            assert_eq!(LogPrefix::Auto.format(true), pff);
            assert_eq!(LogPrefix::Auto.format(false), plf);
        }
    }
}

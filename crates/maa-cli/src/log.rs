use std::{
    io::Write,
    path::{Path, PathBuf},
};

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
    fn to_filter(&self, default_level: log::Level) -> log::LevelFilter {
        use log::LevelFilter::*;
        match (default_level as u8 + self.verbose).saturating_sub(self.quiet) {
            0 => Off,
            1 => Error,
            2 => Warn,
            3 => Info,
            4 => Debug,
            _ => Trace,
        }
    }

    /// Configure the logger without installing it, so the configuration can be tested.
    fn into_builder(
        self,
        default_level: log::Level,
        prefix: LogPrefix,
    ) -> std::io::Result<env_logger::Builder> {
        let level = self.to_filter(default_level);
        let mut builder = env_logger::Builder::new();
        builder
            .filter_level(level)
            // At trace level ureq-proto hex-dumps all HTTP traffic, including query
            // strings, headers and bodies, which can leak credentials. Module filters
            // match by prefix, so `ureq` also covers `ureq_proto`. They override the
            // global level, so never raise it here.
            .filter_module("ureq", level.min(log::LevelFilter::Debug));

        let log_file = log_path(self.log_file);
        let formatter = prefix.formatter(log_file.is_some());
        builder.format(move |buf, record| formatter.format(buf, record));

        if let Some(path) = log_file {
            builder.target(env_logger::Target::Pipe(Box::new(open_log_file(&path)?)));
        }

        Ok(builder)
    }

    pub fn init_logger(self) -> anyhow::Result<()> {
        self.into_builder(default_level(), LogPrefix::from_env())?
            .init();
        Ok(())
    }
}

fn default_level() -> log::Level {
    std::env::var_os("MAA_LOG")
        .and_then(|s| s.to_str().and_then(|s| s.parse().ok()))
        .unwrap_or(log::Level::Warn)
}

fn open_log_file(path: &Path) -> std::io::Result<std::fs::File> {
    if let Some(dir) = path.parent() {
        use crate::dirs::Ensure;
        dir.ensure()?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
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

    fn formatter(&self, log_file: bool) -> LogFormatter {
        match self {
            LogPrefix::Always => LogFormatter::Prefixed,
            LogPrefix::Never => LogFormatter::Plain,
            LogPrefix::Auto => {
                if log_file {
                    LogFormatter::Prefixed
                } else {
                    LogFormatter::Plain
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormatter {
    Prefixed,
    Plain,
}

impl LogFormatter {
    fn format(
        self,
        buf: &mut env_logger::fmt::Formatter,
        record: &log::Record,
    ) -> std::io::Result<()> {
        match self {
            LogFormatter::Prefixed => {
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
            LogFormatter::Plain => {
                writeln!(buf, "{}", record.args())
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    mod level_filters {
        use log::{Level, Log, Metadata};

        use super::*;
        use crate::command::parse_from;

        fn enabled(args: &[&str], target: &str, level: Level) -> bool {
            parse_from(args)
                .log
                .into_builder(Level::Warn, LogPrefix::Never)
                .expect("no log file is opened without --log-file")
                .build()
                .enabled(&Metadata::builder().target(target).level(level).build())
        }

        #[test]
        fn http_trace_is_capped_at_debug() {
            let trace = ["maa", "list", "-vvv"];
            assert!(enabled(&trace, "maa", Level::Trace));
            for target in ["ureq", "ureq::util", "ureq_proto::util"] {
                assert!(!enabled(&trace, target, Level::Trace));
                assert!(enabled(&trace, target, Level::Debug));
            }
        }

        #[test]
        fn http_modules_never_exceed_global_level() {
            assert!(!enabled(&["maa", "list"], "ureq", Level::Info));
            assert!(enabled(&["maa", "list"], "ureq", Level::Warn));
            assert!(!enabled(&["maa", "list", "-q"], "ureq_proto", Level::Warn));
            assert!(!enabled(&["maa", "list", "-qq"], "ureq", Level::Error));
        }
    }

    mod cmd_args {
        use std::env;

        use super::*;
        use crate::command::parse_from;

        #[test]
        fn to_filter() {
            use log::{Level, LevelFilter::*};

            let filter = |args: &[&str]| parse_from(args).log.to_filter(Level::Warn);
            assert_eq!(filter(&["maa", "list"]), Warn);
            assert_eq!(filter(&["maa", "-v", "list"]), Info);
            assert_eq!(filter(&["maa", "list", "-v"]), Info);
            assert_eq!(filter(&["maa", "list", "--verbose"]), Info);
            assert_eq!(filter(&["maa", "--verbose", "list"]), Info);
            assert_eq!(filter(&["maa", "list", "--quiet"]), Error);
            assert_eq!(filter(&["maa", "list", "-vvvv"]), Trace);
            assert_eq!(filter(&["maa", "list", "-vvv"]), Trace);
            assert_eq!(filter(&["maa", "list", "-vv"]), Debug);
            assert_eq!(filter(&["maa", "list", "-vq"]), Warn);
            assert_eq!(filter(&["maa", "list", "-q"]), Error);
            assert_eq!(filter(&["maa", "list", "-qq"]), Off);
            assert_eq!(filter(&["maa", "list", "-qqq"]), Off);

            let args = parse_from(["maa", "list", "-v"]).log;
            assert_eq!(args.to_filter(Level::Error), Warn);
            assert_eq!(args.to_filter(Level::Debug), Trace);
        }

        #[test]
        fn default_level_from_env() {
            use log::Level;

            // Safety: MAA_LOG is only modified and read in this test
            unsafe {
                env::remove_var("MAA_LOG");
                assert_eq!(default_level(), Level::Warn);
                env::set_var("MAA_LOG", "Debug");
                assert_eq!(default_level(), Level::Debug);
                env::set_var("MAA_LOG", "unknown");
                assert_eq!(default_level(), Level::Warn);
                env::remove_var("MAA_LOG");
            }
        }

        #[test]
        fn log_path_from_args() {
            use std::path::Path;
            let log_file = |args: &[&str]| log_path(parse_from(args).log.log_file);
            assert!(log_file(&["maa", "list"]).is_none());
            // The default path depends on the current date; read the clock before and
            // after so the test also passes when it runs across midnight.
            let day_dir = |now: chrono::DateTime<chrono::Local>| {
                crate::dirs::log()
                    .join(now.format("%Y").to_string())
                    .join(now.format("%m").to_string())
                    .join(now.format("%d").to_string())
            };
            let before = chrono::Local::now();
            let path = log_file(&["maa", "list", "--log-file"]).expect("default log path");
            let after = chrono::Local::now();
            assert!(path.starts_with(day_dir(before)) || path.starts_with(day_dir(after)));
            assert!(
                log_file(&["maa", "list", "--log-file=path"])
                    .is_some_and(|x| x == Path::new("path"))
            );
        }

        #[test]
        fn log_file_is_created_and_appended() {
            let dir = tempfile::tempdir().expect("create temp dir");
            let path = dir.path().join("a").join("b.log");
            for content in ["first\n", "second\n"] {
                open_log_file(&path)
                    .and_then(|mut file| file.write_all(content.as_bytes()))
                    .expect("write log file");
            }
            assert_eq!(
                std::fs::read_to_string(&path).expect("read log file"),
                "first\nsecond\n"
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
        fn formatter() {
            assert_eq!(LogPrefix::Always.formatter(true), LogFormatter::Prefixed);
            assert_eq!(LogPrefix::Always.formatter(false), LogFormatter::Prefixed);

            assert_eq!(LogPrefix::Never.formatter(true), LogFormatter::Plain);
            assert_eq!(LogPrefix::Never.formatter(false), LogFormatter::Plain);

            assert_eq!(LogPrefix::Auto.formatter(true), LogFormatter::Prefixed);
            assert_eq!(LogPrefix::Auto.formatter(false), LogFormatter::Plain);
        }
    }
}

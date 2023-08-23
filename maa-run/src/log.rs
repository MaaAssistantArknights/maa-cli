#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error = 0, // Show some error about the program
    Warning,   // Show some warning about the program
    Normal, // Default log level, show some basic info about this program and MaaCore running status
    Info,   // Some additional info about the program, like some detail status of the MaaCore
    Debug, // Detailed info about configuration and others, is used for user to debug thier own configuration
    Trace, // Trace the running of MaaCore, show every failed message processing, used for developer to debug the program
}

impl<T: Into<u8>> From<T> for LogLevel {
    fn from(level: T) -> Self {
        match level.into() {
            0 => Self::Error,
            1 => Self::Warning,
            2 => Self::Normal,
            3 => Self::Info,
            4 => Self::Debug,
            _ => Self::Trace,
        }
    }
}

#[derive(Clone)]
pub struct Logger {
    level: LogLevel,
}

impl Logger {
    pub const fn new(level: LogLevel) -> Self {
        Self { level }
    }

    pub fn set_level(&mut self, level: impl Into<LogLevel>) {
        self.level = level.into();
    }

    pub fn level(&self) -> LogLevel {
        self.level
    }
}

impl<T: Into<LogLevel>> From<T> for Logger {
    fn from(level: T) -> Self {
        let level: LogLevel = level.into();
        Self::new(level)
    }
}

static mut LOGGER: Logger = Logger::new(LogLevel::Normal);

pub unsafe fn set_level(level: impl Into<LogLevel>) {
    LOGGER.set_level(level);
}

pub unsafe fn level() -> LogLevel {
    LOGGER.level()
}

pub fn time() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Show a error message with global log level
///
/// The title will be shown in red color
#[macro_export]
macro_rules! error {
    ($title:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Error {
                eprintln!("{} \x1b[31m{}\x1b[0m", $crate::log::time(), $title);
            }
        }
    };
    ($title:expr, $msg:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Error {
                eprintln!("{} \x1b[31m{}\x1b[0m {}", $crate::log::time(), $title, $msg);
            }
        }
    };
}

/// Show a warning message with global log level
///
/// The title will be shown in yellow color
#[macro_export]
macro_rules! warning {
    ($title:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Warning {
                println!("{} \x1b[33m{}\x1b[0m", $crate::log::time(), $title);
            }
        }
    };
    ($title:expr, $msg:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Warning {
                println!("{} \x1b[33m{}\x1b[0m {}", $crate::log::time(), $title, $msg);
            }
        }
    };
}

/// Show a normal message with global log level
///
/// No special color will be used
#[macro_export]
macro_rules! normal {
    ($title:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Normal {
                println!("{} {}", $crate::log::time(), $title);
            }
        }
    };
    ($title:expr, $msg:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Normal {
                println!("{} {} {}", $crate::log::time(), $title, $msg);
            }
        }
    };
}

/// Show a info message with global log level
///
/// The title will be shown in green color
#[macro_export]
macro_rules! info {
    ($title:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Info {
                println!("{} \x1b[32m{}\x1b[0m", $crate::log::time(), $title);
            }
        }
    };
    ($title:expr, $msg:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Info {
                println!("{} \x1b[32m{}\x1b[0m {}", $crate::log::time(), $title, $msg);
            }
        }
    };
}

/// Show a debug message with global log level
///
/// The title will be shown in blue color
#[macro_export]
macro_rules! debug {
    ($title:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Debug {
                println!("{} \x1b[34m{}\x1b[0m", $crate::log::time(), $title);
            }
        }
    };
    ($title:expr, $msg:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Debug {
                println!("{} \x1b[34m{}\x1b[0m {}", $crate::log::time(), $title, $msg);
            }
        }
    };
}

/// Show a trace message with global log level
///
/// The title will be shown in magenta color
#[macro_export]
macro_rules! trace {
    ($title:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Trace {
                println!("{} \x1b[35m{}\x1b[0m", $crate::log::time(), $title);
            }
        }
    };
    ($title:expr, $msg:expr) => {
        unsafe {
            if $crate::log::level() >= $crate::log::LogLevel::Trace {
                println!("{} \x1b[35m{}\x1b[0m {}", $crate::log::time(), $title, $msg);
            }
        }
    };
}

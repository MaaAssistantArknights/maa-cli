use std::fmt::Display;

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum LogLevel {
    Error = 0,
    Warning,
    Info,
    Debug,
    Trace,
}

impl<T: Into<u8>> From<T> for LogLevel {
    fn from(level: T) -> Self {
        match level.into() {
            0 => Self::Error,
            1 => Self::Warning,
            2 => Self::Info,
            3 => Self::Debug,
            _ => Self::Trace,
        }
    }
}

#[derive(Clone)]
pub struct Logger {
    level: LogLevel,
}

impl Logger {
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }

    pub fn error<F, T, M>(&self, title: T, msg: F)
    where
        F: FnOnce() -> M,
        T: Display,
        M: Display,
    {
        if self.level as u8 >= LogLevel::Error as u8 {
            eprintln!("\x1b[31m{}\x1b[0m {}", title, msg());
        }
    }

    pub fn warning<F, T, M>(&self, title: T, msg: F)
    where
        F: FnOnce() -> M,
        T: Display,
        M: Display,
    {
        if self.level as u8 >= LogLevel::Warning as u8 {
            println!("\x1b[33m{}\x1b[0m {}", title, msg());
        }
    }

    pub fn info<F, T, M>(&self, title: T, msg: F)
    where
        F: FnOnce() -> M,
        T: Display,
        M: Display,
    {
        if self.level as u8 >= LogLevel::Info as u8 {
            println!("\x1b[32m{}\x1b[0m {}", title, msg());
        }
    }

    pub fn debug<F, T, M>(&self, title: T, msg: F)
    where
        F: FnOnce() -> M,
        T: Display,
        M: Display,
    {
        if self.level as u8 >= LogLevel::Debug as u8 {
            println!("\x1b[34m{}\x1b[0m {}", title, msg());
        }
    }
}

impl<T: Into<LogLevel>> From<T> for Logger {
    fn from(level: T) -> Self {
        let level: LogLevel = level.into();
        Self::new(level)
    }
}

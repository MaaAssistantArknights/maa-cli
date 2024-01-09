#[derive(rust_embed::RustEmbed)]
#[folder = "i18n"]
struct I18NAssets;

lazy_static::lazy_static! {
    static ref STATIC_LOADER: i18n_embed::fluent::FluentLanguageLoader = {
        let language_loader = i18n_embed::fluent::fluent_language_loader!();
        let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
        i18n_embed::select(&language_loader, &I18NAssets, &requested_languages)
            .expect("Failed to load language");

        language_loader
    };
}

macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::STATIC_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),* $(,)?) => {{
        i18n_embed_fl::fl!($crate::STATIC_LOADER, $message_id, $($args) *)
    }};
}

// lazy version of fl, create a closure to defer the evaluation of the message.
macro_rules! lfl {
    ($message_id:literal) => {{
        || fl!($message_id)
    }};

    ($message_id:literal, $($args:expr),* $(,)?) => {{
        || fl!($message_id, $($args), *)
    }};
}

/// Print a fluent localized message to stdout with a newline.
macro_rules! printlnfl {
    ($message_id:literal) => {{
        println!("{}", fl!($message_id))
    }};

    ($message_id:literal, $($args:expr),* $(,)?) => {{
        println!("{}", fl!($message_id, $($args), *))
    }};
}

macro_rules! writefl {
    ($f:expr, $message_id:literal) => {{
        write!($f, "{}", fl!($message_id))
    }};
    ($f:expr, $message_id:literal, $($args:expr),* $(,)?) => {{
        write!($f, "{}", fl!($message_id, $($args), *))
    }};
}

macro_rules! writelnfl {
    ($f:expr, $message_id:literal) => {{
        writeln!($f, "{}", fl!($message_id))
    }};
    ($f:expr, $message_id:literal, $($args:expr),* $(,)?) => {{
        writeln!($f, "{}", fl!($message_id, $($args), *))
    }};
}

/// Return a error with a fluent localized message.
macro_rules! bailfl {
    ($message_id:literal) => {{
        anyhow::bail!("{}", fl!($message_id))
    }};

    ($message_id:literal, $($args:expr),* $(,)?) => {{
        anyhow::bail!("{}", fl!($message_id, $($args), *))
    }};
}

// fluent log
macro_rules! trace {
    ($message_id:literal) => {{
        log::trace!("{}", fl!($message_id))
    }};

    ($message_id:literal, $($args:expr),* $(,)?) => {{
        log::trace!("{}", fl!($message_id, $($args), *))
    }};
}

macro_rules! debug {
    ($message_id:literal) => {{
        log::debug!("{}", fl!($message_id))
    }};

    ($message_id:literal, $($args:expr),* $(,)?) => {{
        log::debug!("{}", fl!($message_id, $($args), *))
    }};
}

macro_rules! info {
    ($message_id:literal) => {{
        log::info!("{}", fl!($message_id))
    }};

    ($message_id:literal, $($args:expr),* $(,)?) => {{
        log::info!("{}", fl!($message_id, $($args), *))
    }};
}

macro_rules! warn {
    ($message_id:literal) => {{
        log::warn!("{}", fl!($message_id))
    }};

    ($message_id:literal, $($args:expr),* $(,)?) => {{
        log::warn!("{}", fl!($message_id, $($args), *))
    }};
}

macro_rules! error {
    ($message_id:literal) => {{
        log::error!("{}", fl!($message_id))
    }};

    ($message_id:literal, $($args:expr),* $(,)?) => {{
        log::error!("{}", fl!($message_id, $($args), *))
    }};
}

trait ResultExt<T, E> {
    /// Log the error message if the result is an error.
    ///
    /// If the result is an error, log the error message and return `None`.
    /// Otherwise, return the value in the result.
    fn log_err(self, default: T) -> Option<T>
    where
        E: std::fmt::Display;
}

pub mod activity;
mod command;
mod config;
mod consts;
mod dirs;
mod installer;
mod log;
mod run;
mod value;

fn main() -> anyhow::Result<()> {
    command::process()
}

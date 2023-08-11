use std::path::PathBuf;

#[derive(Debug)]
pub enum Error {
    UnknownFiletype,
    UnsupportedFiletype(String),
    FileNotFound(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Toml(toml::de::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::UnknownFiletype => write!(f, "Unknown filetype"),
            Error::UnsupportedFiletype(s) => write!(f, "Unsupported filetype: {}", s),
            Error::FileNotFound(s) => write!(f, "File not found: {}", s),
            Error::Io(e) => write!(f, "IO error, {}", e),
            Error::Json(e) => write!(f, "JSON parse error, {}", e),
            Error::Toml(e) => write!(f, "TOML parse error, {}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Self {
        Error::Toml(e)
    }
}

pub const SUPPORTED_FILETYPES: [&str; 2] = ["json", "toml"];

pub trait FromFile: Sized + serde::de::DeserializeOwned {
    fn from_file(path: &PathBuf) -> Result<Self, Error> {
        if !path.exists() {
            return Err(Error::FileNotFound(path.to_str().unwrap().to_string()));
        }
        let filetype = path.extension().ok_or(Error::UnknownFiletype)?;
        if filetype == "json" {
            let task_list = serde_json::from_reader(std::fs::File::open(path)?)?;
            Ok(task_list)
        } else if filetype == "toml" {
            let task_list = toml::from_str(&std::fs::read_to_string(path)?)?;
            Ok(task_list)
        } else {
            Err(Error::UnsupportedFiletype(String::from(
                match filetype.to_str() {
                    Some(s) => s,
                    None => "Unknown",
                },
            )))
        }
    }
}

pub trait FindFile: FromFile {
    fn find_file(path: &PathBuf) -> Result<Self, Error> {
        for filetype in SUPPORTED_FILETYPES.iter() {
            let path = path.with_extension(filetype);
            if path.exists() {
                return Self::from_file(&path);
            }
        }
        Err(Error::FileNotFound(path.to_str().unwrap().to_string()))
    }
}

impl<T> FindFile for T where T: FromFile {}

pub mod asst;
pub mod task;

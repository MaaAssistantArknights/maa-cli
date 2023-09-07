use std::path::Path;

#[derive(Debug)]
pub enum Error {
    UnknownFiletype,
    UnsupportedFiletype(String),
    FileNotFound(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Toml(toml::de::Error),
    Yaml(serde_yaml::Error),
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
            Error::Yaml(e) => write!(f, "YAML parse error, {}", e),
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

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Error::Yaml(e)
    }
}

const SUPPORTED_EXTENSION: [&str; 4] = ["json", "yaml", "yml", "toml"];

pub trait FromFile: Sized + serde::de::DeserializeOwned {
    fn from_file(path: &Path) -> Result<Self, Error> {
        if !path.is_file() {
            return Err(Error::FileNotFound(path.to_str().unwrap().to_string()));
        }
        let filetype = path.extension().ok_or(Error::UnknownFiletype)?;
        match filetype.to_str().unwrap() {
            "json" => {
                let task_list = serde_json::from_reader(std::fs::File::open(path)?)?;
                Ok(task_list)
            }
            "toml" => {
                let task_list = toml::from_str(&std::fs::read_to_string(path)?)?;
                Ok(task_list)
            }
            "yml" | "yaml" => {
                let task_list = serde_yaml::from_reader(std::fs::File::open(path)?)?;
                Ok(task_list)
            }
            _ => {
                return Err(Error::UnsupportedFiletype(String::from(
                    filetype.to_str().unwrap_or("Unknown"),
                )))
            }
        }
    }
}

pub trait FindFile: FromFile {
    fn find_file(path: &Path) -> Result<Self, Error> {
        for filetype in SUPPORTED_EXTENSION.iter() {
            let path = path.with_extension(filetype);
            if path.is_file() {
                return Self::from_file(&path);
            }
        }
        Err(Error::FileNotFound(path.to_str().unwrap().to_string()))
    }
}

impl<T> FindFile for T where T: FromFile {}

pub mod asst;
pub mod cli;
pub mod task;

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown filetype")]
    UnknownFiletype,
    #[error("Unsupported filetype: {0}")]
    UnsupportedFiletype(String),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("IO error, {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error, {0}")]
    Json(#[from] serde_json::Error),
    #[error("TOML parse error, {0}")]
    Toml(#[from] toml::de::Error),
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

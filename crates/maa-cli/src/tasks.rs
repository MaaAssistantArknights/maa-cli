use std::path::Path;

use anyhow::Result;
use clap_complete::CompletionCandidate;
use maa_dirs::config;

#[derive(Debug)]
pub struct Tasks {
    names: Vec<String>,
}

impl Tasks {
    pub fn new_with(config_path: &Path) -> Result<Self> {
        let mut names = vec![];

        let task_dir = config_path.join("tasks");
        if task_dir.exists() {
            for entry in task_dir.read_dir()? {
                let path = entry?.path();
                if path.is_file()
                    && let Some(stem) = path.file_stem()
                    && let Some(name) = stem.to_str()
                {
                    names.push(name.to_string());
                }
            }
        }

        Ok(Self { names })
    }

    pub fn new() -> Result<Self> {
        Self::new_with(config())
    }
}

impl Tasks {
    pub fn completer(current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
        let mut completions = vec![];

        if let Some(current) = current.to_str()
            && let Ok(Self { names }) = Self::new()
        {
            for name in &names {
                if name.starts_with(current) {
                    completions.push(CompletionCandidate::new(name));
                }
            }
        }

        completions
    }
}

impl std::fmt::Display for Tasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for name in &self.names {
            writeln!(f, "{name}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use std::fs::{self, File};

    use super::*;

    #[test]
    fn test_tasks_dir_not_exists() {
        let tmp = tempfile::tempdir().unwrap();

        let tasks = Tasks::new_with(tmp.path()).unwrap();
        assert!(tasks.names.is_empty());
    }

    #[test]
    fn test_read_task_files() {
        let tmp = tempfile::tempdir().unwrap();

        let task_dir = tmp.path().join("tasks");
        fs::create_dir(&task_dir).unwrap();

        File::create(task_dir.join("a.toml")).unwrap();
        File::create(task_dir.join("b.json")).unwrap();

        let Tasks { names } = Tasks::new_with(tmp.path()).unwrap();

        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
    }

    #[test]
    fn test_display() {
        let tasks = Tasks {
            names: vec!["a".into(), "b".into()],
        };

        let output = format!("{}", tasks);

        assert_eq!(output, "a\nb\n");
    }
}

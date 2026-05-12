use anyhow::Result;
use clap_complete::CompletionCandidate;
use maa_dirs::config;

#[derive(Debug)]
pub struct Tasks {
    names: Vec<String>,
}

impl Tasks {
    pub fn new() -> Result<Self> {
        let mut names = vec![];

        let task_dir = config().join("tasks");
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

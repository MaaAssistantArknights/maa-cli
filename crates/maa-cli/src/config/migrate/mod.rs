//! Migrate configuration from external models into maa-cli config shapes.
//!
//! Unlike [`super::convert`], which only changes serialization format for the same
//! data structure, migration may select profiles, remap fields, and drop unsupported
//! content. Results are therefore potentially lossy and always accompanied by a summary.

mod wpf;

use std::path::Path;

use anyhow::{Context, Result, bail};
use maa_value::value::MAAValue;

use super::{Filetype, FromFile};

/// Summary of lossy choices made during migration.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MigrationSummary {
    /// Unsupported task types that were not written.
    pub skipped_tasks: Vec<SkippedTask>,
    /// Tasks with `IsEnable = false`, kept inactive in the output.
    pub disabled_tasks: Vec<SkippedTask>,
    /// Fields present on a supported task that were not mapped.
    pub skipped_fields: Vec<SkippedField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedTask {
    pub type_tag: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedField {
    pub task_type: String,
    pub task_name: Option<String>,
    pub field: String,
}

impl MigrationSummary {
    pub fn is_empty(&self) -> bool {
        self.skipped_tasks.is_empty()
            && self.disabled_tasks.is_empty()
            && self.skipped_fields.is_empty()
    }

    fn skip_task(&mut self, type_tag: impl Into<String>, name: Option<String>) {
        self.skipped_tasks.push(SkippedTask {
            type_tag: type_tag.into(),
            name,
        });
    }

    fn disable_task(&mut self, type_tag: impl Into<String>, name: Option<String>) {
        self.disabled_tasks.push(SkippedTask {
            type_tag: type_tag.into(),
            name,
        });
    }

    fn skip_field(
        &mut self,
        task_type: impl Into<String>,
        task_name: Option<String>,
        field: impl Into<String>,
    ) {
        self.skipped_fields.push(SkippedField {
            task_type: task_type.into(),
            task_name,
            field: field.into(),
        });
    }

    /// Print a user-facing summary of lossy migration decisions to stderr.
    pub fn print(&self) {
        if self.is_empty() {
            return;
        }

        eprintln!("Migration summary (lossy):");

        if !self.skipped_tasks.is_empty() {
            eprintln!("  Skipped tasks (unsupported type):");
            for task in &self.skipped_tasks {
                eprintln!("    - {}", format_task_ref(task));
            }
        }

        if !self.disabled_tasks.is_empty() {
            eprintln!("  Disabled tasks (kept inactive with never-true condition; will not run):");
            for task in &self.disabled_tasks {
                eprintln!("    - {}", format_task_ref(task));
            }
        }

        if !self.skipped_fields.is_empty() {
            eprintln!("  Skipped fields (not migrated):");
            for field in &self.skipped_fields {
                match &field.task_name {
                    Some(name) if !name.is_empty() => {
                        eprintln!("    - {} \"{}\": {}", field.task_type, name, field.field);
                    }
                    _ => {
                        eprintln!("    - {}: {}", field.task_type, field.field);
                    }
                }
            }
        }
    }
}

fn format_task_ref(task: &SkippedTask) -> String {
    match &task.name {
        Some(name) if !name.is_empty() => format!("{} \"{name}\"", task.type_tag),
        _ => task.type_tag.clone(),
    }
}

/// Migrate a MAA WPF GUI profile into a maa-cli task config.
///
/// ```text
/// maa migrate wpf <input> [output]
/// ```
pub fn wpf(
    file: &Path,
    out: Option<&Path>,
    ft: Option<Filetype>,
    config_name: Option<&str>,
) -> Result<()> {
    use maa_dirs::Ensure;

    if !matches!(Filetype::parse_filetype(file), Some(Filetype::Json)) {
        log::warn!(
            "`maa migrate wpf` expects a MAA WPF GUI profile (typically JSON); \
             input {} is not JSON",
            file.display()
        );
    }

    let ft = ft.or_else(|| {
        out.and_then(|path| path.extension())
            .and_then(|ext| ext.to_str())
            .and_then(Filetype::parse_extension)
    });

    let value = MAAValue::from_file(file)?;
    let value = wpf::select_configuration(value, config_name)?;
    let (value, summary) = wpf::migrate(value)?;

    let Some(format) = ft else {
        bail!("Format not given")
    };

    if let Some(file) = out {
        let file = file.with_extension(format.to_str());
        if let Some(dir) = file.parent() {
            dir.ensure()?;
        }
        format
            .write(&file, &value)
            .with_context(|| format!("Failed to write migrated file {}", file.display()))?;
    } else {
        format.write_to(std::io::stdout().lock(), &value)?;
    }

    summary.print();
    Ok(())
}

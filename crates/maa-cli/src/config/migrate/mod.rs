//! Migrate configuration from external models into maa-cli config shapes.
//!
//! Unlike [`super::convert`], which only changes serialization format for the same
//! data structure, migration may select profiles, remap fields, and drop unsupported
//! content. Results are therefore potentially lossy and always accompanied by a summary.

use std::fmt;

mod wpf;

pub(crate) use wpf::wpf;

/// Summary of lossy choices made during migration.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MigrationSummary {
    /// Unsupported task types that were not written.
    pub skipped_tasks: Vec<SkippedTask>,
    /// Tasks with `IsEnable = false`, written with `params.enable = false`.
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

    pub(super) fn skip_task(&mut self, type_tag: impl Into<String>, name: Option<String>) {
        self.skipped_tasks.push(SkippedTask {
            type_tag: type_tag.into(),
            name,
        });
    }

    pub(super) fn disable_task(&mut self, type_tag: impl Into<String>, name: Option<String>) {
        self.disabled_tasks.push(SkippedTask {
            type_tag: type_tag.into(),
            name,
        });
    }

    pub(super) fn skip_field(
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
}

impl fmt::Display for MigrationSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return Ok(());
        }

        writeln!(f, "Migration summary (lossy):")?;

        if !self.skipped_tasks.is_empty() {
            writeln!(f, "  Skipped tasks (unsupported type):")?;
            for task in &self.skipped_tasks {
                writeln!(f, "    - {}", format_task_ref(task))?;
            }
        }

        if !self.disabled_tasks.is_empty() {
            writeln!(
                f,
                "  Disabled tasks (kept with params.enable=false; will not run):"
            )?;
            for task in &self.disabled_tasks {
                writeln!(f, "    - {}", format_task_ref(task))?;
            }
        }

        if !self.skipped_fields.is_empty() {
            writeln!(f, "  Skipped fields (not migrated):")?;
            for field in &self.skipped_fields {
                match &field.task_name {
                    Some(name) if !name.is_empty() => {
                        writeln!(f, "    - {} \"{name}\": {}", field.task_type, field.field)?;
                    }
                    _ => {
                        writeln!(f, "    - {}: {}", field.task_type, field.field)?;
                    }
                }
            }
        }

        Ok(())
    }
}

fn format_task_ref(task: &SkippedTask) -> String {
    match &task.name {
        Some(name) if !name.is_empty() => format!("{} \"{name}\"", task.type_tag),
        _ => task.type_tag.clone(),
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn display_formats_skipped_task_with_name() {
        let summary = MigrationSummary {
            skipped_tasks: vec![SkippedTask {
                type_tag: "UserDataUpdateTask".into(),
                name: Some("sync".into()),
            }],
            ..Default::default()
        };
        let text = summary.to_string();
        assert!(text.starts_with("Migration summary (lossy):\n"));
        assert!(text.contains("UserDataUpdateTask \"sync\""));
    }

    #[test]
    fn display_is_empty_for_empty_summary() {
        assert!(MigrationSummary::default().to_string().is_empty());
    }
}

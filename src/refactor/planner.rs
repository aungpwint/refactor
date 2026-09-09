use crate::context::RepoContext;
use crate::error::{RefactorError, Result};
use crate::output::Output;
use crate::refactor::replacement;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct MigrationPlan {
    pub version: u32,
    pub operations: Vec<MigrationOperation>,
}

#[derive(Deserialize)]
pub struct MigrationOperation {
    #[serde(rename = "type")]
    pub op_type: String,
    pub from: String,
    pub to: String,
}

pub fn load_plan(path: &PathBuf) -> Result<MigrationPlan> {
    let content = std::fs::read_to_string(path)?;
    let plan: MigrationPlan = if path.to_string_lossy().ends_with(".json") {
        serde_json::from_str(&content)?
    } else {
        toml::from_str(&content)?
    };

    if plan.version != 1 {
        return Err(RefactorError::MigrationPlan(format!(
            "Unsupported plan version: {}",
            plan.version
        )));
    }

    Ok(plan)
}

pub fn validate_plan(_ctx: &RepoContext, plan: &MigrationPlan) -> Result<()> {
    for (i, op) in plan.operations.iter().enumerate() {
        match op.op_type.as_str() {
            "replace" | "rename" | "import" => {}
            other => {
                return Err(RefactorError::MigrationPlan(format!(
                    "Unknown operation type '{}' at index {}",
                    other, i
                )));
            }
        }

        if op.from == op.to {
            return Err(RefactorError::MigrationPlan(format!(
                "Operation {} has identical from/to: {}",
                i, op.from
            )));
        }
    }
    Ok(())
}

pub fn execute_plan(
    ctx: &RepoContext,
    output: &Output,
    plan: &MigrationPlan,
    dry_run: bool,
) -> Result<()> {
    for (i, op) in plan.operations.iter().enumerate() {
        output.info(&format!(
            "Operation {}/{}: {} → {}",
            i + 1,
            plan.operations.len(),
            op.from,
            op.to
        ));

        match op.op_type.as_str() {
            "replace" => {
                let replace_plan =
                    replacement::plan_replacement(ctx, &op.from, &op.to, false, false, false)?;
                replacement::show_plan(output, &replace_plan, dry_run);
                if !dry_run {
                    let result = replacement::execute_replacement(ctx, &replace_plan, false)?;
                    output.success(&format!("{} files changed", result.files_changed));
                }
            }
            "import" => {
                let imp_plan = crate::refactor::import_migration::plan_import_migration(
                    ctx, &op.from, &op.to,
                )?;
                output.info(&format!(
                    "  {} imports would be updated",
                    imp_plan.imports_updated
                ));
            }
            "rename" => {
                let old_path = ctx.root.join(&op.from);
                let new_path = ctx.root.join(&op.to);
                let rename_plan = crate::refactor::rename::plan_rename(ctx, &old_path, &new_path)?;
                crate::refactor::rename::show_rename_plan(output, &rename_plan, dry_run);
                if !dry_run {
                    let result = crate::refactor::rename::execute_rename(ctx, &rename_plan, false)?;
                    output.success(&format!("{} files updated", result.files_changed));
                }
            }
            _ => {}
        }
        output.blank();
    }
    Ok(())
}

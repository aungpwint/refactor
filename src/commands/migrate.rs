use crate::cli::MigrateArgs;
use crate::context::RepoContext;
use crate::error::{RefactorError, Result};
use crate::exec::ExecOptions;
use crate::output::Output;
use crate::refactor::planner;

pub fn run(
    ctx: &RepoContext,
    output: &Output,
    args: &MigrateArgs,
    opts: &ExecOptions,
) -> Result<i32> {
    let dry_run = opts.dry_run;

    output.heading("Migration");

    if !args.plan.exists() {
        return Err(RefactorError::MigrationPlan(format!(
            "Plan file not found: {}",
            args.plan.display()
        )));
    }

    output.info(&format!("Plan: {}", args.plan.display()));

    output.progress("Loading migration plan");
    let plan = planner::load_plan(&args.plan)?;
    output.progress_done();

    output.info(&format!("Operations: {}", plan.operations.len()));

    output.progress("Validating plan");
    planner::validate_plan(ctx, &plan)?;
    output.progress_done();

    output.success("Plan is valid");

    if dry_run {
        output.blank();
        output.info("Dry run — showing planned operations:");
        output.blank();
    }

    planner::execute_plan(ctx, output, &plan, dry_run)?;

    if dry_run {
        output.blank();
        output.info("Dry run complete — no changes were made.");
    } else {
        output.success("Migration complete");
    }

    Ok(0)
}

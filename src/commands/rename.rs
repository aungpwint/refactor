use crate::cli::RenameArgs;
use crate::context::RepoContext;
use crate::error::Result;
use crate::output::Output;
use crate::refactor::rename;

pub fn run(ctx: &RepoContext, output: &Output, args: &RenameArgs) -> Result<i32> {
    let dry_run = std::env::args().any(|a| a == "--dry-run");

    output.heading("Rename");
    output.progress("Planning rename");
    let plan = rename::plan_rename(ctx, &args.old, &args.new)?;
    output.progress_done();

    rename::show_rename_plan(output, &plan, dry_run);

    if dry_run {
        return Ok(0);
    }

    if !std::env::args().any(|a| a == "--yes") && !dry_run {
        output.warn("Rename requires --yes to confirm (or use --dry-run)");
        return Ok(2);
    }

    output.progress("Executing rename");
    let result = rename::execute_rename(ctx, &plan, false)?;
    output.progress_done();

    output.blank();
    output.success(&format!("{} files updated", result.files_changed));
    if result.errors > 0 {
        output.warn(&format!("{} files failed", result.errors));
    }
    output.success(&format!(
        "Renamed {} → {}",
        plan.source_relative, plan.dest_relative
    ));

    Ok(0)
}

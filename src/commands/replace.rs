use crate::cli::ReplaceArgs;
use crate::context::RepoContext;
use crate::error::Result;
use crate::output::Output;
use crate::refactor::replacement;

pub fn run(ctx: &RepoContext, output: &Output, args: &ReplaceArgs) -> Result<i32> {
    let dry_run = std::env::args().any(|a| a == "--dry-run");

    output.heading("Replace");
    output.info(&format!("Old: \"{}\"", args.old));
    output.info(&format!("New: \"{}\"", args.new));
    output.blank();

    output.progress("Scanning repository");
    let plan = replacement::plan_replacement(
        ctx,
        &args.old,
        &args.new,
        args.regex,
        args.case_sensitive,
        args.whole_word,
    )?;
    output.progress_done();

    output.verbose_msg(&format!(
        "Pattern: {} (regex={}, case_sensitive={}, whole_word={})",
        args.old, args.regex, args.case_sensitive, args.whole_word
    ));

    if dry_run {
        output.blank();
    } else if ctx.is_git && ctx.has_dirty_worktree() && !args.allow_dirty {
        output.warn("Git working tree is dirty — modified/untracked files may be overwritten.");
        output.info("Use --allow-dirty to silence this warning.");
        output.blank();
    }

    if plan.changes.is_empty() {
        output.success("No matches found. Nothing to replace.");
        return Ok(0);
    }

    replacement::show_plan(output, &plan, dry_run);

    if dry_run {
        output.print_json_result(&crate::output::ScanResult {
            command: "replace".to_string(),
            files_scanned: 0,
            files_changed: Some(plan.files_affected),
            replacements: Some(plan.replacements),
            errors: None,
            skipped: None,
            data: None,
        });
        return Ok(0);
    }

    output.progress("Applying changes");
    let result = replacement::execute_replacement(ctx, &plan, false)?;
    output.progress_done();

    output.blank();
    output.success(&format!("{} files updated", result.files_changed));
    if result.errors > 0 {
        output.warn(&format!("{} errors occurred", result.errors));
    }

    output.print_json_result(&crate::output::ScanResult {
        command: "replace".to_string(),
        files_scanned: 0,
        files_changed: Some(result.files_changed),
        replacements: Some(plan.replacements),
        errors: Some(result.errors),
        skipped: None,
        data: None,
    });

    if result.errors > 0 {
        Ok(6)
    } else {
        Ok(0)
    }
}

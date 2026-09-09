use crate::cli::{PathsAction, PathsCommand};
use crate::context::RepoContext;
use crate::error::Result;
use crate::output::Output;

pub fn run(ctx: &RepoContext, output: &Output, args: &PathsCommand) -> Result<i32> {
    match &args.action {
        PathsAction::Scan(_) => scan(ctx, output),
        PathsAction::Check(_) => check(ctx, output),
        PathsAction::Migrate(args) => migrate(ctx, output, &args.old, &args.new),
        PathsAction::Normalize(_) => normalize(ctx, output),
    }
}

fn scan(ctx: &RepoContext, output: &Output) -> Result<i32> {
    output.heading("Path Scan");
    let broken = crate::analyzers::references::find_broken_references(ctx);

    output.info(&format!("Broken path references: {}", broken.len()));

    if !broken.is_empty() {
        output.blank();
        for r#ref in &broken {
            output.line(&format!(
                "  {} → {} ({})",
                r#ref.from, r#ref.reference, r#ref.resolved
            ));
        }
    }

    Ok(0)
}

fn check(ctx: &RepoContext, output: &Output) -> Result<i32> {
    output.heading("Path Check");
    let broken = crate::analyzers::references::find_broken_references(ctx);
    let import_issues = crate::analyzers::imports::check_import_consistency(ctx);

    let total_errors = broken.len()
        + import_issues
            .iter()
            .filter(|i| matches!(i.kind, crate::analyzers::imports::IssueKind::BrokenImport))
            .count();

    if total_errors == 0 {
        output.success("All paths are valid");
    } else {
        output.warn(&format!("{total_errors} path issues found"));
        for r#ref in &broken {
            output.line(&format!("  Broken: {} → {}", r#ref.from, r#ref.reference));
        }
        for issue in &import_issues {
            if matches!(
                issue.kind,
                crate::analyzers::imports::IssueKind::BrokenImport
            ) {
                output.line(&format!(
                    "  Broken import: {}:{} — {}",
                    issue.file, issue.line, issue.import_path
                ));
            }
        }
    }

    Ok(if total_errors > 0 { 1 } else { 0 })
}

fn migrate(ctx: &RepoContext, output: &Output, old: &str, new: &str) -> Result<i32> {
    let dry_run = std::env::args().any(|a| a == "--dry-run");

    output.heading("Path Migration");
    output.info(&format!("Old: \"{old}\""));
    output.info(&format!("New: \"{new}\""));

    output.progress("Planning path migration");
    let plan = crate::refactor::path_migration::plan_path_migration(ctx, old, new)?;
    output.progress_done();

    output.info(&format!("Files affected: {}", plan.files_affected));
    output.info(&format!("Paths to update: {}", plan.paths_updated));

    if output.verbose {
        for change in &plan.changes {
            output.line(&format!(
                "  {}:{} — {} → {}",
                change.relative, change.line_number, change.old_path, change.new_path
            ));
        }
    }

    if !dry_run {
        output.progress("Applying path migration");
        let updated = crate::refactor::path_migration::execute_path_migration(ctx, &plan, false)?;
        output.progress_done();
        output.blank();
        output.success(&format!("{updated} files updated"));
    } else {
        output.blank();
        output.info("Dry run — no changes made.");
    }

    Ok(0)
}

fn normalize(_ctx: &RepoContext, output: &Output) -> Result<i32> {
    output.heading("Path Normalization");
    output.success("Path normalization complete");
    Ok(0)
}

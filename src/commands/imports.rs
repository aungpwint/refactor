use crate::cli::{ImportsAction, ImportsCommand};
use crate::context::RepoContext;
use crate::error::Result;
use crate::exec::ExecOptions;
use crate::output::Output;
use crate::scanner::content_scanner::scan_imports;

pub fn run(
    ctx: &RepoContext,
    output: &Output,
    args: &ImportsCommand,
    opts: &ExecOptions,
) -> Result<i32> {
    match &args.action {
        ImportsAction::Scan(_) => scan(ctx, output),
        ImportsAction::Check(_) => check(ctx, output),
        ImportsAction::Migrate(args) => migrate(ctx, output, &args.old, &args.new, opts),
        ImportsAction::Normalize(_) => normalize(ctx, output),
        ImportsAction::Unused(_) => unused(ctx, output),
    }
}

fn scan(ctx: &RepoContext, output: &Output) -> Result<i32> {
    output.heading("Import Scan");

    output.progress("Scanning imports");
    let file_imports = scan_imports(ctx);
    output.progress_done();

    let total_imports: usize = file_imports.iter().map(|f| f.imports.len()).sum();
    let total_exports: usize = file_imports.iter().map(|f| f.exports.len()).sum();

    output.info(&format!("Files with imports: {}", file_imports.len()));
    output.info(&format!("Total imports: {total_imports}"));
    output.info(&format!("Total exports: {total_exports}"));

    if !file_imports.is_empty() {
        output.blank();
        output.info("Sample imports:");
        for file in file_imports.iter().take(10) {
            for imp in file.imports.iter().take(3) {
                let kind = import_kind(&imp.import_type);
                output.line(&format!(
                    "  {}:{} [{kind}] — {}",
                    file.file.relative, imp.line_number, imp.import_path
                ));
            }
        }
    }

    if output.json_mode {
        let data = serde_json::json!({
            "files_with_imports": file_imports.len(),
            "total_imports": total_imports,
            "total_exports": total_exports,
        });
        output.print_json_result(&crate::output::ScanResult {
            command: "imports scan".to_string(),
            files_scanned: file_imports.len(),
            files_changed: None,
            replacements: None,
            errors: None,
            skipped: None,
            data: Some(data),
        });
    }

    Ok(0)
}

fn check(ctx: &RepoContext, output: &Output) -> Result<i32> {
    output.heading("Import Check");
    let issues = crate::analyzers::imports::check_import_consistency(ctx);

    if issues.is_empty() {
        output.success("All imports are consistent");
    } else {
        output.warn(&format!("{} issues found", issues.len()));
        for issue in &issues {
            output.line(&format!(
                "  {}:{} — {} ({})",
                issue.file, issue.line, issue.import_path, issue.detail
            ));
        }
    }

    Ok(if issues.is_empty() { 0 } else { 1 })
}

fn migrate(
    ctx: &RepoContext,
    output: &Output,
    old: &str,
    new: &str,
    opts: &ExecOptions,
) -> Result<i32> {
    let dry_run = opts.dry_run;

    output.heading("Import Migration");
    output.info(&format!("Old: \"{old}\""));
    output.info(&format!("New: \"{new}\""));

    output.progress("Planning import migration");
    let plan = crate::refactor::import_migration::plan_import_migration(ctx, old, new)?;
    output.progress_done();

    output.info(&format!("Files affected: {}", plan.files_affected));
    output.info(&format!("Imports to update: {}", plan.imports_updated));

    if output.verbose {
        for change in &plan.changes {
            output.line(&format!(
                "  {}:{} — {} → {}",
                change.relative, change.line_number, change.old_import, change.new_import
            ));
            output.line(&format!("     {}", change.new_line.trim()));
        }
    }

    if !dry_run {
        output.progress("Applying import migration");
        let updated =
            crate::refactor::import_migration::execute_import_migration(ctx, &plan, false)?;
        output.progress_done();
        output.blank();
        output.success(&format!("{updated} files updated"));
    } else {
        output.blank();
        output.info("Dry run — no changes made.");
    }

    Ok(0)
}

fn import_kind(kind: &crate::languages::ImportType) -> &'static str {
    match kind {
        crate::languages::ImportType::Static => "static",
        crate::languages::ImportType::Dynamic => "dynamic",
        crate::languages::ImportType::Require => "require",
        crate::languages::ImportType::ReExport => "reexport",
    }
}

fn normalize(ctx: &RepoContext, output: &Output) -> Result<i32> {
    output.heading("Import Normalization");
    let issues = crate::analyzers::imports::check_import_consistency(ctx);

    let normalizable: Vec<_> = issues
        .iter()
        .filter(|i| {
            matches!(
                i.kind,
                crate::analyzers::imports::IssueKind::InconsistentSlashes
                    | crate::analyzers::imports::IssueKind::DoubleSlash
                    | crate::analyzers::imports::IssueKind::RedundantSegment
            )
        })
        .collect();

    if normalizable.is_empty() {
        output.success("Imports are already normalized");
    } else {
        output.info(&format!("{} imports can be normalized", normalizable.len()));
    }

    Ok(0)
}

fn unused(ctx: &RepoContext, output: &Output) -> Result<i32> {
    output.heading("Unused Imports");
    let file_imports = scan_imports(ctx);

    let mut total_unused = 0;
    for file in &file_imports {
        for imp in &file.imports {
            if imp.import_path.contains("unused") || imp.import_path.contains("test") {
                output.line(&format!(
                    "  {}:{} — possibly unused: {}",
                    file.file.relative, imp.line_number, imp.import_path
                ));
                total_unused += 1;
            }
        }
    }

    if total_unused == 0 {
        output.success("No obviously unused imports detected");
    } else {
        output.info(&format!("{} possibly unused imports", total_unused));
    }

    Ok(0)
}

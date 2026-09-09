use crate::analyzers::consistency::check_repository_consistency;
use crate::analyzers::duplicates::find_duplicates;
use crate::cli::CheckArgs;
use crate::context::RepoContext;
use crate::error::Result;
use crate::output::Output;

pub fn run(ctx: &RepoContext, output: &Output, args: &CheckArgs) -> Result<i32> {
    output.heading("Checking repository consistency");

    output.progress("Analyzing imports and references");
    let report = check_repository_consistency(ctx);
    output.progress_done();

    output.progress("Checking for duplicates");
    let duplicates = find_duplicates(ctx, 1);
    output.progress_done();

    output.blank();
    output.heading("Results");

    if !report.import_issues.is_empty() {
        output.warn(&format!(
            "{} import issues found",
            report.import_issues.len()
        ));
        for issue in &report.import_issues {
            output.line(&format!(
                "  {}:{} — {} ({})",
                issue.file, issue.line, issue.import_path, issue.detail
            ));
        }
    } else {
        output.success("No import issues found");
    }

    output.blank();

    if !report.broken_references.is_empty() {
        output.warn(&format!(
            "{} broken references found",
            report.broken_references.len()
        ));
        for r#ref in &report.broken_references {
            output.line(&format!(
                "  {} → {} (resolved: {})",
                r#ref.from, r#ref.reference, r#ref.resolved
            ));
        }
    } else {
        output.success("No broken references found");
    }

    output.blank();

    if !duplicates.is_empty() {
        output.warn(&format!("{} duplicate groups found", duplicates.len()));
        for group in &duplicates {
            output.line(&format!("  Hash: {}", &group.hash[..12]));
            for file in &group.files {
                output.line(&format!("    {file}"));
            }
        }
    } else {
        output.success("No duplicate files found");
    }

    let total_errors = report.total_errors + duplicates.len();
    let total_warnings = report.total_warnings;

    output.blank();
    if total_errors == 0 && total_warnings == 0 {
        output.success("Repository is consistent");
    } else {
        output.info(&format!(
            "Errors: {total_errors}, Warnings: {total_warnings}"
        ));
    }

    if output.json_mode {
        let data = serde_json::json!({
            "import_issues": report.import_issues.len(),
            "broken_references": report.broken_references.len(),
            "duplicate_groups": duplicates.len(),
            "total_errors": total_errors,
            "total_warnings": total_warnings,
        });
        output.print_json_result(&crate::output::ScanResult {
            command: "check".to_string(),
            files_scanned: 0,
            files_changed: None,
            replacements: None,
            errors: Some(total_errors),
            skipped: None,
            data: Some(data),
        });
    }

    if total_errors > 0 || (args.strict && total_warnings > 0) {
        Ok(1)
    } else {
        Ok(0)
    }
}

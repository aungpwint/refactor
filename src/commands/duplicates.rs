use crate::analyzers::duplicates::find_duplicates;
use crate::cli::DuplicatesArgs;
use crate::context::RepoContext;
use crate::error::Result;
use crate::output::Output;

pub fn run(ctx: &RepoContext, output: &Output, args: &DuplicatesArgs) -> Result<i32> {
    let min_size = args.min_size.unwrap_or(1);

    output.heading("Duplicate Files");

    output.progress("Scanning for duplicates");
    let duplicates = find_duplicates(ctx, min_size);
    output.progress_done();

    if duplicates.is_empty() {
        output.success("No duplicate files found");
    } else {
        output.warn(&format!("{} duplicate groups:", duplicates.len()));
        for group in &duplicates {
            output.blank();
            output.line(&format!(
                "  Hash: {}",
                &group.hash[..12.min(group.hash.len())]
            ));
            for file in &group.files {
                output.line(&format!("    {file}"));
            }
        }
    }

    if output.json_mode {
        let data = serde_json::json!({
            "duplicate_groups": duplicates.len(),
            "groups": duplicates.iter().map(|g| serde_json::json!({
                "hash": g.hash,
                "files": g.files,
            })).collect::<Vec<_>>(),
        });
        output.print_json_result(&crate::output::ScanResult {
            command: "duplicates".to_string(),
            files_scanned: 0,
            files_changed: None,
            replacements: None,
            errors: None,
            skipped: None,
            data: Some(data),
        });
    }

    Ok(0)
}

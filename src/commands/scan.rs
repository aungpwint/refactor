use crate::cli::ScanArgs;
use crate::context::RepoContext;
use crate::error::Result;
use crate::output::Output;
use crate::scanner::file_scanner::scan_repository;

pub fn run(ctx: &RepoContext, output: &Output, _args: &ScanArgs) -> Result<i32> {
    output.heading("Repository");
    output.info(&format!("Root: {}", ctx.root.display()));
    output.blank();

    output.progress("Scanning repository");
    let stats = scan_repository(ctx);
    output.progress_done();

    output.heading("Files");
    let mut exts: Vec<_> = stats.by_extension.iter().collect();
    exts.sort_by(|a, b| b.1.cmp(a.1));
    for (ext, count) in &exts {
        output.info(&format!("{ext:>12}: {count}"));
    }
    output.info(&format!("{:>12}: {}", "Total", stats.total_files));
    output.blank();

    output.heading("Directories");
    output.info(&format!("{:>12}: {}", "Total", stats.total_dirs));
    output.info(&format!("{:>12}: {}", "Source", stats.source_files));
    output.blank();

    if output.json_mode {
        let data = serde_json::json!({
            "total_files": stats.total_files,
            "total_dirs": stats.total_dirs,
            "source_files": stats.source_files,
            "by_extension": stats.by_extension,
        });
        output.print_json_result(&crate::output::ScanResult {
            command: "scan".to_string(),
            files_scanned: stats.total_files,
            files_changed: None,
            replacements: None,
            errors: None,
            skipped: None,
            data: Some(data),
        });
    } else {
        output.success(&format!("Scanned {} files", stats.total_files));
    }

    Ok(0)
}

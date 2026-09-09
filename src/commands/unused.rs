use crate::analyzers::references::find_unreferenced_files;
use crate::cli::UnusedArgs;
use crate::context::RepoContext;
use crate::error::Result;
use crate::output::Output;

pub fn run(ctx: &RepoContext, output: &Output, _args: &UnusedArgs) -> Result<i32> {
    output.heading("Unused Files");

    output.progress("Analyzing references");
    let unreferenced = find_unreferenced_files(ctx);
    output.progress_done();

    if unreferenced.is_empty() {
        output.success("No obviously unused files detected");
    } else {
        output.warn(&format!("{} potentially unused files:", unreferenced.len()));
        for file in &unreferenced {
            output.line(&format!("  {file}"));
        }
    }

    if output.json_mode {
        let data = serde_json::json!({
            "unused_files": unreferenced,
            "count": unreferenced.len(),
        });
        output.print_json_result(&crate::output::ScanResult {
            command: "unused".to_string(),
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

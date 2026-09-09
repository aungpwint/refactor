use crate::cli::ReferencesArgs;
use crate::context::RepoContext;
use crate::error::Result;
use crate::filesystem::reader::read_file_content;
use crate::languages::detect_language;
use crate::output::Output;
use crate::scanner::reference_scanner::build_reference_graph;

pub fn run(ctx: &RepoContext, output: &Output, args: &ReferencesArgs) -> Result<i32> {
    output.heading("References");
    output.info(&format!("Target: {}", args.path));

    output.progress("Building reference graph");
    let graph = build_reference_graph(ctx);
    output.progress_done();

    let referencing_files = graph.find_references_to(&args.path);
    output.blank();

    if referencing_files.is_empty() {
        output.info("No references found.");
    } else {
        output.info(&format!("Found {} references:", referencing_files.len()));
    }

    let mut total_refs = 0usize;
    for (i, reference) in referencing_files.iter().enumerate() {
        output.line(&format!("  {}. {}", i + 1, reference));

        let full_path = ctx.root.join(reference);
        if let Ok(content) = read_file_content(&full_path) {
            let lang = detect_language(&full_path);
            for r#ref in lang.analyze_references(&content.text) {
                if r#ref.text.contains(&args.path) {
                    let kind = match r#ref.ref_type {
                        crate::languages::ReferenceType::Import => "import",
                        crate::languages::ReferenceType::Export => "export",
                        crate::languages::ReferenceType::Require => "require",
                        crate::languages::ReferenceType::DynamicImport => "dynamic",
                        crate::languages::ReferenceType::PathString => "path",
                        crate::languages::ReferenceType::UrlPath => "url",
                    };
                    output.line(&format!(
                        "     line {} [{kind}]: {}",
                        r#ref.line_number,
                        r#ref.line_content.trim()
                    ));
                    total_refs += 1;
                }
            }
        }
    }

    if output.json_mode {
        let data = serde_json::json!({
            "target": args.path,
            "references": referencing_files,
            "count": referencing_files.len(),
        });
        output.print_json_result(&crate::output::ScanResult {
            command: "references".to_string(),
            files_scanned: 0,
            files_changed: None,
            replacements: Some(total_refs),
            errors: None,
            skipped: None,
            data: Some(data),
        });
    }

    Ok(0)
}

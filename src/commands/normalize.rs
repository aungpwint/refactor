use crate::cli::NormalizeArgs;
use crate::context::RepoContext;
use crate::error::Result;
use crate::output::Output;

pub fn run(ctx: &RepoContext, output: &Output, _args: &NormalizeArgs) -> Result<i32> {
    output.heading("Normalize");

    let issues = crate::analyzers::imports::check_import_consistency(ctx);

    let mut normalized = 0;
    for issue in &issues {
        match issue.kind {
            crate::analyzers::imports::IssueKind::InconsistentSlashes
            | crate::analyzers::imports::IssueKind::DoubleSlash
            | crate::analyzers::imports::IssueKind::RedundantSegment => {
                normalized += 1;
            }
            _ => {}
        }
    }

    if normalized == 0 {
        output.success("Nothing to normalize");
    } else {
        output.info(&format!("{normalized} items could be normalized"));
    }

    Ok(0)
}

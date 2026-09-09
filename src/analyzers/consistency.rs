use crate::analyzers::imports::check_import_consistency;
use crate::analyzers::references::find_broken_references;
use crate::context::RepoContext;

pub fn check_repository_consistency(ctx: &RepoContext) -> ConsistencyReport {
    let import_issues = check_import_consistency(ctx);
    let broken_refs = find_broken_references(ctx);

    let mut errors = 0;
    let mut warnings = 0;

    for issue in &import_issues {
        match issue.kind {
            crate::analyzers::imports::IssueKind::BrokenImport => errors += 1,
            _ => warnings += 1,
        }
    }
    errors += broken_refs.len();

    ConsistencyReport {
        import_issues,
        broken_references: broken_refs,
        total_errors: errors,
        total_warnings: warnings,
    }
}

pub struct ConsistencyReport {
    pub import_issues: Vec<crate::analyzers::imports::ImportIssue>,
    pub broken_references: Vec<crate::analyzers::references::BrokenReference>,
    pub total_errors: usize,
    pub total_warnings: usize,
}

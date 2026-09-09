use crate::analyzers::tsconfig::{read_path_aliases, PathAliases};
use crate::context::RepoContext;
use crate::scanner::content_scanner::{scan_imports, FileImports};
use crate::utils::paths::normalize_slashes;

#[derive(Clone)]
#[allow(dead_code)]
pub enum IssueKind {
    BrokenImport,
    DoubleSlash,
    RedundantSegment,
    InconsistentSlashes,
    CaseMismatch,
    AliasMismatch,
    CircularDependency,
}

#[derive(Clone)]
pub struct ImportIssue {
    pub file: String,
    pub line: usize,
    pub import_path: String,
    pub kind: IssueKind,
    pub detail: String,
}

pub fn check_import_consistency(ctx: &RepoContext) -> Vec<ImportIssue> {
    let file_imports = scan_imports(ctx);
    let aliases = read_path_aliases(ctx);
    let mut issues = Vec::new();

    for file in &file_imports {
        for imp in &file.imports {
            check_import(ctx, file, imp, &aliases, &mut issues);
        }
    }

    issues
}

fn check_import(
    ctx: &RepoContext,
    file: &FileImports,
    imp: &crate::languages::ImportInfo,
    aliases: &PathAliases,
    issues: &mut Vec<ImportIssue>,
) {
    let path_str = normalize_slashes(&imp.import_path);

    if path_str.starts_with('.') {
        let resolved =
            crate::analyzers::references::resolve_relative(&file.file.relative, &path_str);
        if !crate::analyzers::references::might_exist(ctx, &resolved) {
            issues.push(ImportIssue {
                file: file.file.relative.clone(),
                line: imp.line_number,
                import_path: imp.import_path.clone(),
                kind: IssueKind::BrokenImport,
                detail: format!("Resolved path does not exist: {resolved}"),
            });
        }
    }

    if let Some((alias, target)) = match_alias(&path_str, aliases) {
        let resolved = resolve_alias(&path_str, &alias, target);
        if !crate::analyzers::references::might_exist(ctx, &resolved) {
            issues.push(ImportIssue {
                file: file.file.relative.clone(),
                line: imp.line_number,
                import_path: imp.import_path.clone(),
                kind: IssueKind::AliasMismatch,
                detail: format!("Alias '{}' resolves to missing path: {resolved}", alias),
            });
        }
    }

    if path_str.contains("//") {
        issues.push(ImportIssue {
            file: file.file.relative.clone(),
            line: imp.line_number,
            import_path: imp.import_path.clone(),
            kind: IssueKind::DoubleSlash,
            detail: "Import path contains double slashes".to_string(),
        });
    }

    if path_str.contains("/./") {
        issues.push(ImportIssue {
            file: file.file.relative.clone(),
            line: imp.line_number,
            import_path: imp.import_path.clone(),
            kind: IssueKind::RedundantSegment,
            detail: "Import path contains redundant ./ segment".to_string(),
        });
    }

    if !path_str.starts_with('.') && !looks_like_alias(&path_str, aliases) {
        if let Some(case_issue) = check_case_sensitivity(ctx, &path_str, &file.file.relative) {
            issues.push(case_issue);
        }
    }
}

fn match_alias(path: &str, aliases: &PathAliases) -> Option<(String, Vec<String>)> {
    for (pattern, targets) in aliases {
        if let Some(alias) = pattern.strip_suffix("/*") {
            if path == alias || path.starts_with(&format!("{alias}/")) {
                return Some((alias.to_string(), targets.clone()));
            }
        } else if path == pattern {
            return Some((pattern.clone(), targets.clone()));
        }
    }
    None
}

fn resolve_alias(path: &str, alias: &str, targets: Vec<String>) -> String {
    let rest = path.strip_prefix(alias).unwrap_or(path);
    let Some(relative) = targets.first() else {
        return path.to_string();
    };
    let base = relative.trim_start_matches("./").trim_end_matches("/*");
    let base = base.trim_end_matches('*');
    format!("{base}{rest}")
}

fn looks_like_alias(path: &str, aliases: &PathAliases) -> bool {
    for pattern in aliases.keys() {
        let alias = pattern.trim_end_matches("/*").trim_end_matches('*');
        if path == alias || path.starts_with(&format!("{alias}/")) {
            return true;
        }
    }
    false
}

fn check_case_sensitivity(ctx: &RepoContext, path: &str, from_file: &str) -> Option<ImportIssue> {
    let resolved = crate::analyzers::references::resolve_relative(from_file, path);
    let full = ctx.root.join(&resolved);
    if full.exists() {
        return None;
    }

    let actual = find_case_variant(ctx, &resolved)?;
    if actual != resolved {
        return Some(ImportIssue {
            file: from_file.to_string(),
            line: 0,
            import_path: path.to_string(),
            kind: IssueKind::CaseMismatch,
            detail: format!(
                "Import casing '{}' does not match actual file '{}'",
                resolved, actual
            ),
        });
    }
    None
}

fn find_case_variant(ctx: &RepoContext, resolved: &str) -> Option<String> {
    let parent = std::path::Path::new(resolved).parent()?;
    let file_name = std::path::Path::new(resolved).file_name()?.to_str()?;
    let dir = ctx.root.join(parent);
    if !dir.is_dir() {
        return None;
    }
    let entries = std::fs::read_dir(&dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.to_lowercase() == file_name.to_lowercase() && name != file_name {
            return Some(format!("{}/{name}", parent.to_string_lossy()));
        }
    }
    None
}

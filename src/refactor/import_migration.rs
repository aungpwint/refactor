use crate::context::RepoContext;
use crate::error::Result;
use crate::filesystem::reader::{is_binary_file, read_file_content, restore_bom};
use crate::filesystem::walker::collect_files;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct ImportMigrationPlan {
    pub files_affected: usize,
    pub imports_updated: usize,
    pub changes: Vec<ImportChange>,
}

pub struct ImportChange {
    pub path: PathBuf,
    pub relative: String,
    pub old_import: String,
    pub new_import: String,
    pub line_number: usize,
    pub new_line: String,
}

pub fn plan_import_migration(
    ctx: &RepoContext,
    old: &str,
    new: &str,
) -> Result<ImportMigrationPlan> {
    let files = collect_files(ctx);

    let changes: Vec<ImportChange> = files
        .into_par_iter()
        .filter_map(|entry| {
            if is_binary_file(&entry.path) {
                return None;
            }
            let content = read_file_content(&entry.path).ok()?;
            let lang = crate::languages::detect_language(&entry.path);
            let imports = lang.analyze_imports(&content.text, &entry.path);

            let mut file_changes = Vec::new();
            for imp in &imports {
                if imp.import_path == old || imp.import_path.starts_with(&format!("{old}/")) {
                    let replacement = if imp.import_path == old {
                        new.to_string()
                    } else {
                        format!("{}{}", new, &imp.import_path[old.len()..])
                    };
                    file_changes.push(ImportChange {
                        path: entry.path.clone(),
                        relative: entry.relative.clone(),
                        old_import: imp.import_path.clone(),
                        new_import: replacement.clone(),
                        line_number: imp.line_number,
                        new_line: imp.line_content.replace(&imp.import_path, &replacement),
                    });
                }
            }

            if file_changes.is_empty() {
                return None;
            }

            Some(file_changes)
        })
        .flatten()
        .collect();

    let total = changes.len();
    Ok(ImportMigrationPlan {
        files_affected: changes
            .iter()
            .map(|c| c.path.clone())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        imports_updated: total,
        changes,
    })
}

pub fn execute_import_migration(
    _ctx: &RepoContext,
    plan: &ImportMigrationPlan,
    dry_run: bool,
) -> Result<usize> {
    if dry_run {
        return Ok(plan.imports_updated);
    }

    let mut per_file: HashMap<&PathBuf, Vec<&ImportChange>> = HashMap::new();
    for change in &plan.changes {
        per_file.entry(&change.path).or_default().push(change);
    }

    let mut updated = 0;
    for (path, changes) in per_file {
        let content = read_file_content(path)?;
        let new_text = apply_line_edits(&content.text, &changes);
        if new_text != content.text {
            let mut bytes = new_text.as_bytes().to_vec();
            restore_bom(content.has_bom, &mut bytes);
            crate::filesystem::writer::write_file_safe(path, &bytes)?;
            updated += 1;
        }
    }

    Ok(updated)
}

fn apply_line_edits(text: &str, changes: &[&ImportChange]) -> String {
    let mut result = String::with_capacity(text.len());
    let mut change_idx = 0;

    for (current_line, segment) in (1usize..).zip(text.split_inclusive('\n')) {
        if let Some(change) = changes.get(change_idx) {
            if change.line_number == current_line {
                let line_without_terminator = segment.trim_end_matches(['\r', '\n']);
                let terminator = &segment[line_without_terminator.len()..];
                let new_line =
                    line_without_terminator.replace(&change.old_import, &change.new_import);
                result.push_str(&new_line);
                result.push_str(terminator);
            } else {
                result.push_str(segment);
            }
            if change.line_number <= current_line {
                change_idx += 1;
            }
        } else {
            result.push_str(segment);
        }
    }

    result
}

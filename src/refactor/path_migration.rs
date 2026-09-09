use crate::context::RepoContext;
use crate::error::Result;
use crate::filesystem::reader::{is_binary_file, read_file_content, restore_bom};
use crate::filesystem::walker::collect_files;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct PathMigrationPlan {
    pub files_affected: usize,
    pub paths_updated: usize,
    pub changes: Vec<PathChange>,
}

pub struct PathChange {
    pub path: PathBuf,
    pub relative: String,
    pub old_path: String,
    pub new_path: String,
    pub line_number: usize,
}

pub fn plan_path_migration(ctx: &RepoContext, old: &str, new: &str) -> Result<PathMigrationPlan> {
    let files = collect_files(ctx);

    let changes: Vec<PathChange> = files
        .into_par_iter()
        .filter_map(|entry| {
            if is_binary_file(&entry.path) {
                return None;
            }
            let content = read_file_content(&entry.path).ok()?;
            let mut file_changes = Vec::new();

            for (i, line) in content.text.lines().enumerate() {
                if line.contains(old) {
                    file_changes.push(PathChange {
                        path: entry.path.clone(),
                        relative: entry.relative.clone(),
                        old_path: old.to_string(),
                        new_path: new.to_string(),
                        line_number: i + 1,
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
    Ok(PathMigrationPlan {
        files_affected: changes
            .iter()
            .map(|c| c.path.clone())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        paths_updated: total,
        changes,
    })
}

pub fn execute_path_migration(
    _ctx: &RepoContext,
    plan: &PathMigrationPlan,
    dry_run: bool,
) -> Result<usize> {
    if dry_run {
        return Ok(plan.paths_updated);
    }

    let mut per_file: HashMap<&PathBuf, &PathChange> = HashMap::new();
    for change in &plan.changes {
        per_file.entry(&change.path).or_insert(change);
    }

    let mut updated = 0;
    for (path, change) in per_file {
        let content = read_file_content(path)?;
        let new_text = content.text.replace(&change.old_path, &change.new_path);
        if new_text != content.text {
            let mut bytes = new_text.as_bytes().to_vec();
            restore_bom(content.has_bom, &mut bytes);
            crate::filesystem::writer::write_file_safe(path, &bytes)?;
            updated += 1;
        }
    }

    Ok(updated)
}

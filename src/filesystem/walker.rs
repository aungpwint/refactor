use crate::context::RepoContext;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FileEntry {
    pub path: PathBuf,
    pub relative: String,
}

pub fn collect_files(ctx: &RepoContext) -> Vec<FileEntry> {
    let root = ctx.root.clone();
    let exclude_dirs = ctx.exclude_dirs.clone();
    let include_dirs = ctx.include_dirs.clone();
    let exts = ctx.extensions.clone();

    let mut builder = WalkBuilder::new(&root);
    builder.hidden(false).git_ignore(true).require_git(false);

    let ext_lower: Vec<String> = exts.iter().map(|e| e.to_lowercase()).collect();

    builder
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|e| !is_in_ignored_parent(e.path(), &exclude_dirs))
        .filter(|e| {
            e.path()
                .strip_prefix(&root)
                .ok()
                .map(|rel| is_included(rel, &include_dirs))
                .unwrap_or(true)
        })
        .filter(|e| {
            if ext_lower.is_empty() {
                true
            } else {
                e.path()
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| ext_lower.iter().any(|ext| ext == &x.to_lowercase()))
                    .unwrap_or(false)
            }
        })
        .filter_map(|e| {
            let rel = e.path().strip_prefix(&root).ok()?;
            Some(FileEntry {
                path: e.path().to_path_buf(),
                relative: rel.to_string_lossy().to_string(),
            })
        })
        .collect()
}

pub fn collect_dirs(ctx: &RepoContext) -> Vec<PathBuf> {
    let root = ctx.root.clone();
    let exclude_dirs = ctx.exclude_dirs.clone();
    let include_dirs = ctx.include_dirs.clone();

    WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir())
        .filter(|e| {
            e.path() != root.as_path()
                && !is_in_ignored_parent(e.path(), &exclude_dirs)
                && e.path()
                    .strip_prefix(&root)
                    .map(|rel| is_included(rel, &include_dirs))
                    .unwrap_or(true)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn is_included(relative: &Path, include_dirs: &[String]) -> bool {
    if include_dirs.is_empty() {
        return true;
    }
    let rel = relative.to_string_lossy().replace('\\', "/");
    let rel = rel.trim_start_matches("./");
    include_dirs.iter().any(|inc| {
        let inc = inc.replace('\\', "/");
        let inc = inc.trim().trim_matches('/').to_string();
        rel == inc || rel.starts_with(&format!("{inc}/"))
    })
}

fn is_in_ignored_parent(path: &Path, ignores: &[String]) -> bool {
    let parts: Vec<String> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str().map(|s| s.to_string()))
        .collect();
    for ignore in ignores {
        let ignore_parts: Vec<&str> = ignore.split('/').filter(|s| !s.is_empty()).collect();
        if ignore_parts.is_empty() {
            continue;
        }
        if parts.len() < ignore_parts.len() {
            continue;
        }
        for window in parts.windows(ignore_parts.len()) {
            if window.iter().zip(ignore_parts.iter()).all(|(a, b)| a == b) {
                return true;
            }
        }
    }
    false
}

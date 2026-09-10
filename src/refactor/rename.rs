use crate::context::RepoContext;
use crate::error::{RefactorError, Result};
use crate::filesystem::reader::is_binary_file;
use crate::filesystem::walker::collect_files;
use crate::output::Output;
use rayon::prelude::*;
use std::path::PathBuf;

pub struct RenamePlan {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub source_relative: String,
    pub dest_relative: String,
    pub affected_files: Vec<AffectedFile>,
    pub references_to_update: usize,
}

pub struct AffectedFile {
    pub path: PathBuf,
    pub relative: String,
    pub reference_count: usize,
    pub old_specifier: String,
    pub new_specifier: String,
}

pub fn plan_rename(ctx: &RepoContext, old: &PathBuf, new: &PathBuf) -> Result<RenamePlan> {
    let source = if old.is_absolute() {
        old.clone()
    } else {
        ctx.root.join(old)
    };

    let destination = if new.is_absolute() {
        new.clone()
    } else {
        ctx.root.join(new)
    };

    if !source.exists() {
        return Err(RefactorError::Validation(format!(
            "Source path does not exist: {}",
            source.display()
        )));
    }

    if destination.exists() {
        return Err(RefactorError::Conflict(format!(
            "Destination already exists: {}",
            destination.display()
        )));
    }

    if !ctx.is_in_root(&source) {
        return Err(RefactorError::PathSafety(format!(
            "Source path is outside repository root: {}",
            source.display()
        )));
    }

    if !ctx.is_in_root(&destination) {
        return Err(RefactorError::PathSafety(format!(
            "Destination path is outside repository root: {}",
            destination.display()
        )));
    }

    let source_relative = ctx
        .relative_path(&source)
        .unwrap_or_else(|| source.display().to_string());
    let dest_relative = ctx
        .relative_path(&destination)
        .unwrap_or_else(|| destination.display().to_string());

    let files = collect_files(ctx);
    let source_rel = source_relative.replace('\\', "/");
    let dest_rel = dest_relative.replace('\\', "/");
    let source_rel_noext = strip_source_ext(&source_rel);
    let dest_rel_noext = strip_source_ext(&dest_rel);
    let source_stem: PathBuf = source_rel_noext.clone().into();

    let affected: Vec<AffectedFile> = files
        .par_iter()
        .filter_map(|entry| {
            if entry.path == source || is_binary_file(&entry.path) {
                return None;
            }
            let content = std::fs::read_to_string(&entry.path).ok()?;

            let from_dir = std::path::Path::new(&entry.relative)
                .parent()
                .unwrap_or_else(|| std::path::Path::new(""));
            let old_specifier = relative_specifier(from_dir, &source_rel_noext);
            let new_specifier = relative_specifier(from_dir, &dest_rel_noext);

            let candidates =
                unique_candidates(&source_rel, &source_rel_noext, &old_specifier, &source_stem);
            let count = candidates
                .iter()
                .map(|c| count_references(&content, c))
                .sum();
            if count == 0 {
                return None;
            }
            Some(AffectedFile {
                path: entry.path.clone(),
                relative: entry.relative.clone(),
                reference_count: count,
                old_specifier,
                new_specifier,
            })
        })
        .collect();

    let total_refs: usize = affected.iter().map(|a| a.reference_count).sum();

    Ok(RenamePlan {
        source,
        destination,
        source_relative,
        dest_relative,
        affected_files: affected,
        references_to_update: total_refs,
    })
}

fn strip_source_ext(rel: &str) -> String {
    let p = std::path::Path::new(rel);
    let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
        return rel.to_string();
    };
    for ext in ["tsx", "ts", "jsx", "js", "mjs", "cjs"] {
        if let Some(stripped) = name.strip_suffix(&format!(".{ext}")) {
            let base = p
                .parent()
                .map(|d| d.to_string_lossy().to_string())
                .unwrap_or_default();
            let prefix = if base.is_empty() {
                String::new()
            } else {
                format!("{base}/")
            };
            return format!("{prefix}{stripped}");
        }
    }
    rel.to_string()
}

fn relative_specifier(from_dir: &std::path::Path, target: &str) -> String {
    pathdiff_rel(from_dir, std::path::Path::new(target))
}

fn pathdiff_rel(from_dir: &std::path::Path, target: &std::path::Path) -> String {
    let from_parts: Vec<String> = from_dir
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let tgt_parts: Vec<String> = target
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    let common = from_parts
        .iter()
        .zip(tgt_parts.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut out = Vec::new();
    for _ in common..from_parts.len() {
        out.push("..".to_string());
    }
    for part in &tgt_parts[common..] {
        out.push(part.clone());
    }
    if out.is_empty() {
        return ".".to_string();
    }
    format!("./{}", out.join("/"))
}

fn unique_candidates(
    src: &str,
    src_noext: &str,
    rel: &str,
    _stem: &std::path::Path,
) -> Vec<String> {
    let mut out = Vec::new();
    out.push(src_noext.to_string());
    out.push(src.to_string());
    out.push(rel.to_string());
    out.dedup();
    out
}

pub fn execute_rename(
    _ctx: &RepoContext,
    plan: &RenamePlan,
    dry_run: bool,
) -> Result<crate::refactor::result::ChangeResult> {
    let mut files_changed = 0;
    let mut errors = 0;

    if !dry_run {
        for affected in &plan.affected_files {
            match update_references_in_file(
                &affected.path,
                &affected.old_specifier,
                &affected.new_specifier,
            ) {
                Ok(changed) => {
                    if changed {
                        files_changed += 1;
                    }
                }
                Err(_) => errors += 1,
            }
        }

        if let Some(parent) = plan.destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Err(e) = std::fs::rename(&plan.source, &plan.destination) {
            return Err(RefactorError::Filesystem(e));
        }
    } else {
        files_changed = plan.affected_files.len();
    }

    Ok(crate::refactor::result::ChangeResult {
        files_changed,
        errors,
    })
}

fn count_references(content: &str, pattern: &str) -> usize {
    content.matches(pattern).count()
}

fn update_references_in_file(path: &PathBuf, old: &str, new: &str) -> Result<bool> {
    let content = std::fs::read_to_string(path)?;
    let new_content = replace_path_specifiers(&content, old, new);
    if new_content == content {
        return Ok(false);
    }
    crate::filesystem::writer::write_file_safe(path, new_content.as_bytes())?;
    Ok(true)
}

fn replace_path_specifiers(content: &str, old: &str, new: &str) -> String {
    let old_esc = regex::escape(old);
    // Capture the opening quote and closing boundary together.
    let re = regex::Regex::new(&format!(r#"(["']){old_esc}(["'; \t\n])"#))
        .expect("valid specifier regex");
    re.replace_all(content, |caps: &regex::Captures| {
        format!("{}{}{}", &caps[1], new, &caps[2])
    })
    .to_string()
}

pub fn show_rename_plan(output: &Output, plan: &RenamePlan, dry_run: bool) {
    if dry_run {
        output.heading("DRY RUN — Rename");
    } else {
        output.heading("Rename Plan");
    }

    output.info(&format!("Source:      {}", plan.source_relative));
    output.info(&format!("Destination: {}", plan.dest_relative));
    output.blank();
    output.info(&format!("Affected files: {}", plan.affected_files.len()));
    output.info(&format!(
        "References to update: {}",
        plan.references_to_update
    ));

    if !plan.affected_files.is_empty() {
        output.blank();
        output.info("Files to update:");
        for affected in &plan.affected_files {
            output.line(&format!(
                "  {} ({} references)",
                affected.relative, affected.reference_count
            ));
        }
    }

    output.blank();
    if dry_run {
        output.info("No changes were made.");
    }
}

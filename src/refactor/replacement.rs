use crate::context::RepoContext;
use crate::error::Result;
use crate::filesystem::reader::{is_binary_file, read_file_content, restore_bom};
use crate::filesystem::walker::collect_files;
use crate::output::Output;
use rayon::prelude::*;
use regex::Regex;
use std::path::PathBuf;

pub struct ReplacePlan {
    pub files_affected: usize,
    pub replacements: usize,
    pub changes: Vec<FileChange>,
}

pub struct FileChange {
    pub path: PathBuf,
    pub relative: String,
    pub replacements: usize,
    pub new_content: String,
    pub has_bom: bool,
}

pub fn plan_replacement(
    ctx: &RepoContext,
    old: &str,
    new: &str,
    regex_mode: bool,
    case_sensitive: bool,
    whole_word: bool,
) -> Result<ReplacePlan> {
    let files = collect_files(ctx);
    let pattern = build_pattern(old, regex_mode, case_sensitive, whole_word)?;

    let changes: Vec<FileChange> = files
        .par_iter()
        .filter_map(|entry| {
            if is_binary_file(&entry.path) {
                return None;
            }
            let content = read_file_content(&entry.path).ok()?;
            let (new_text, count) = apply_replacement(&content.text, &pattern, new)?;
            if count == 0 {
                return None;
            }
            Some(FileChange {
                path: entry.path.clone(),
                relative: entry.relative.clone(),
                replacements: count,
                new_content: new_text,
                has_bom: content.has_bom,
            })
        })
        .collect();

    let total_replacements: usize = changes.iter().map(|c| c.replacements).sum();

    Ok(ReplacePlan {
        files_affected: changes.len(),
        replacements: total_replacements,
        changes,
    })
}

pub fn execute_replacement(
    _ctx: &RepoContext,
    plan: &ReplacePlan,
    dry_run: bool,
) -> Result<crate::refactor::result::ChangeResult> {
    let mut changed = 0;
    let mut errors = 0;

    for change in &plan.changes {
        if dry_run {
            changed += 1;
            continue;
        }
        let mut bytes = change.new_content.as_bytes().to_vec();
        restore_bom(change.has_bom, &mut bytes);
        match crate::filesystem::writer::write_file_safe(&change.path, &bytes) {
            Ok(()) => changed += 1,
            Err(_) => errors += 1,
        }
    }

    Ok(crate::refactor::result::ChangeResult {
        files_changed: changed,
        errors,
    })
}

fn build_pattern(
    old: &str,
    regex_mode: bool,
    case_sensitive: bool,
    whole_word: bool,
) -> Result<Pattern> {
    if regex_mode {
        let flags = if case_sensitive { "" } else { "(?i)" };
        let pattern_str = if whole_word {
            format!("{flags}\\b{old}\\b")
        } else {
            format!("{flags}{old}")
        };
        let re = Regex::new(&pattern_str)?;
        Ok(Pattern::Regex(re))
    } else {
        Ok(Pattern::Literal {
            old: old.to_string(),
            case_sensitive,
            whole_word,
        })
    }
}

enum Pattern {
    Regex(Regex),
    Literal {
        old: String,
        case_sensitive: bool,
        whole_word: bool,
    },
}

fn apply_replacement(text: &str, pattern: &Pattern, new: &str) -> Option<(String, usize)> {
    match pattern {
        Pattern::Regex(re) => {
            let count = re.find_iter(text).count();
            if count == 0 {
                return None;
            }
            let result = re.replace_all(text, new).to_string();
            Some((result, count))
        }
        Pattern::Literal {
            old,
            case_sensitive,
            whole_word,
        } => {
            let (result, count) = if *whole_word {
                literal_replace_whole_word(text, old, new, *case_sensitive)
            } else if *case_sensitive {
                literal_replace(text, old, new)
            } else {
                literal_replace_case_insensitive(text, old, new)
            };
            if count == 0 {
                None
            } else {
                Some((result, count))
            }
        }
    }
}

fn literal_replace(text: &str, old: &str, new: &str) -> (String, usize) {
    let count = text.matches(old).count();
    let result = text.replace(old, new);
    (result, count)
}

fn literal_replace_case_insensitive(text: &str, old: &str, new: &str) -> (String, usize) {
    let re = Regex::new(&format!(r"(?i){}", regex::escape(old)))
        .unwrap_or_else(|_| Regex::new(&regex::escape(old)).expect("escaped pattern is valid"));
    let count = re.find_iter(text).count();
    if count == 0 {
        return (text.to_string(), 0);
    }
    (re.replace_all(text, new).to_string(), count)
}

fn literal_replace_whole_word(
    text: &str,
    old: &str,
    new: &str,
    _case_sensitive: bool,
) -> (String, usize) {
    let pattern_str = format!(r"(?i)\b{}\b", regex::escape(old));
    let re = Regex::new(&pattern_str).ok().unwrap();
    let count = re.find_iter(text).count();
    if count == 0 {
        return (text.to_string(), 0);
    }
    (re.replace_all(text, new).to_string(), count)
}

pub fn show_plan(output: &Output, plan: &ReplacePlan, dry_run: bool) {
    if dry_run {
        output.heading("DRY RUN");
    } else {
        output.heading("Plan");
    }

    output.info(&format!("Files affected: {}", plan.files_affected));
    output.info(&format!("Replacements: {}", plan.replacements));
    output.blank();

    if !plan.changes.is_empty() {
        output.info("Files:");
        for change in &plan.changes {
            output.line(&format!(
                "  {} ({} replacements)",
                change.relative, change.replacements
            ));
        }
    }

    output.blank();
    if dry_run {
        output.info("No files were changed.");
    }
}

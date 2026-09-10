use crate::cli::CleanArgs;
use crate::context::RepoContext;
use crate::error::Result;
use crate::exec::ExecOptions;
use crate::output::Output;
use std::path::Path;

pub fn run(
    ctx: &RepoContext,
    output: &Output,
    args: &CleanArgs,
    opts: &ExecOptions,
) -> Result<i32> {
    output.heading("Clean");

    let dry_run = opts.dry_run;
    let preview = if dry_run { " (dry run)" } else { "" };

    let mut total_cleaned = 0;

    if args.temp_files {
        output.info(&format!("Scanning for temporary files{preview}..."));
        let cleaned = clean_temp_files(ctx, dry_run)?;
        total_cleaned += cleaned;
        output.info(&format!("Removed {cleaned} temporary files"));
    }

    if args.cache {
        output.info(&format!("Scanning for cache files{preview}..."));
        let cleaned = clean_cache_files(ctx, dry_run)?;
        total_cleaned += cleaned;
        output.info(&format!("Removed {cleaned} cache files"));
    }

    if args.empty_dirs {
        output.info(&format!("Scanning for empty directories{preview}..."));
        let cleaned = clean_empty_dirs(ctx, dry_run)?;
        total_cleaned += cleaned;
        output.info(&format!("Removed {cleaned} empty directories"));
    }

    if !args.temp_files && !args.cache && !args.empty_dirs {
        output.info("No cleanup targets specified.");
        output.info("Use --empty-dirs, --temp-files, or --cache to specify targets.");
    }

    if dry_run {
        output.info("Dry run — no changes were made.");
    } else if total_cleaned == 0 {
        output.success("Nothing to clean");
    } else {
        output.success(&format!("Cleaned {total_cleaned} items"));
    }

    Ok(0)
}

fn clean_temp_files(ctx: &RepoContext, dry_run: bool) -> Result<usize> {
    let mut count = 0;
    for entry in walkdir::WalkDir::new(&ctx.root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let name = entry.file_name().to_string_lossy();
        if (name.ends_with(".tmp") || name.ends_with(".bak") || name.starts_with(".~"))
            && entry.file_type().is_file()
        {
            if !dry_run {
                std::fs::remove_file(entry.path()).ok();
            }
            count += 1;
        }
    }
    Ok(count)
}

fn clean_cache_files(ctx: &RepoContext, dry_run: bool) -> Result<usize> {
    let mut count = 0;
    let cache_dirs = [".cache", "__pycache__", ".pytest_cache"];
    for entry in walkdir::WalkDir::new(&ctx.root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_dir() {
            let name = entry.file_name().to_string_lossy();
            if cache_dirs.contains(&name.as_ref()) {
                if !dry_run {
                    std::fs::remove_dir_all(entry.path()).ok();
                }
                count += 1;
            }
        }
    }
    Ok(count)
}

fn clean_empty_dirs(ctx: &RepoContext, dry_run: bool) -> Result<usize> {
    let mut count = 0;
    let dirs = crate::filesystem::walker::collect_dirs(ctx);
    let mut sorted_dirs = dirs;
    sorted_dirs.sort_by_key(|b| std::cmp::Reverse(b.components().count()));

    for dir in &sorted_dirs {
        if dir == &ctx.root {
            continue;
        }
        if is_ignored_dir(dir, &ctx.exclude_dirs) {
            continue;
        }
        if dir.exists() && is_dir_empty(dir) {
            if !dry_run {
                std::fs::remove_dir(dir).ok();
            }
            count += 1;
        }
    }
    Ok(count)
}

fn is_dir_empty(path: &Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true)
}

fn is_ignored_dir(path: &Path, ignores: &[String]) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    ignores
        .iter()
        .any(|i| i == name || i.trim_end_matches('/') == name)
}

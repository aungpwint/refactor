use crate::cli::StripCommentsArgs;
use crate::context::RepoContext;
use crate::error::{RefactorError, Result};
use crate::exec::ExecOptions;
use crate::filesystem::reader::{is_binary_file, read_file_content, restore_bom};
use crate::filesystem::walker::{collect_files_with_extensions, FileEntry};
use crate::filesystem::writer::write_file_safe;
use crate::languages::comments::{self, CommentStyle, Options, Selection};
use crate::output::Output;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct FileChange {
    path: PathBuf,
    relative: String,
    comments: usize,
    new_content: String,
    has_bom: bool,
    style: CommentStyle,
}

struct Plan {
    scanned: usize,
    skipped_generated: usize,
    changes: Vec<FileChange>,
    comments: usize,
}

/// What the per-file pass found. A generated file is reported rather than
/// dropped so the summary can say how many were held back.
enum FileOutcome {
    Stripped(FileChange),
    Generated,
    Unchanged,
}

pub fn run(
    ctx: &RepoContext,
    output: &Output,
    args: &StripCommentsArgs,
    opts: &ExecOptions,
) -> Result<i32> {
    let selection = comments::resolve(&args.lang)
        .map_err(|e| RefactorError::Validation(format!("--lang: {e}")))?;

    output.heading("Strip comments");
    output.info(&format!(
        "Languages: {}",
        selection
            .styles
            .iter()
            .map(|s| s.label())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    output.info(&format!("Extensions: {}", selection.extensions.join(", ")));
    output.info("Kept: //go:build, //go:embed, //go:generate, //nolint, // rustfmt::skip");
    output.info("Kept: /// <reference>, @ts-ignore, eslint-*, prettier-ignore, sourceMappingURL");
    output.info("Kept: GraphQL descriptions, which are schema rather than commentary");
    if args.strip_directives {
        output.info("Directives: also being removed (--strip-directives)");
    }
    output.blank();

    let files = collect_files_with_extensions(ctx, &comments::extensions_for(&selection));
    if files.is_empty() {
        output.warn("No matching files found.");
        return Ok(0);
    }

    output.progress("Scanning repository");
    let plan = plan(&files, &selection, args);
    output.progress_done();

    output.verbose_msg(&format!("{} files scanned", plan.scanned));

    if !opts.dry_run && ctx.is_git && ctx.has_dirty_worktree() && !args.allow_dirty {
        output.warn("Git working tree is dirty — modified/untracked files may be overwritten.");
        output.info("Use --allow-dirty to silence this warning.");
        output.blank();
    }

    if plan.changes.is_empty() {
        output.success("No comments found. Nothing to strip.");
        return Ok(0);
    }

    show_plan(output, &plan, opts.dry_run);

    if opts.dry_run {
        report(output, &plan, true, 0);
        return Ok(0);
    }

    output.progress("Applying changes");
    let (written, errors) = apply(&plan, args, output);
    output.progress_done();

    output.blank();
    output.success(&format!("{written} files updated"));
    if plan.skipped_generated > 0 {
        output.info(&format!(
            "{} generated file(s) skipped; use --generated to include them",
            plan.skipped_generated
        ));
    }
    if errors > 0 {
        output.warn(&format!("{errors} errors occurred"));
    }

    report(output, &plan, false, errors);
    Ok(if errors > 0 { 6 } else { 0 })
}

fn plan(files: &[FileEntry], selection: &Selection, args: &StripCommentsArgs) -> Plan {
    let engine_opts = Options {
        keep_blank_lines: args.keep_blank_lines,
        strip_directives: args.strip_directives,
    };

    let outcomes: Vec<FileOutcome> = files
        .par_iter()
        .filter(|entry| !is_binary_file(&entry.path))
        .filter_map(|entry| {
            let style = comments::style_for(&entry.path, selection)?;
            let content = read_file_content(&entry.path).ok()?;

            if !args.generated && comments::is_generated(&content.text) {
                return Some(FileOutcome::Generated);
            }

            let result = comments::strip(style, &content.text, &engine_opts);
            if result.comments == 0 || result.content == content.text {
                return Some(FileOutcome::Unchanged);
            }

            Some(FileOutcome::Stripped(FileChange {
                path: entry.path.clone(),
                relative: entry.relative.clone(),
                comments: result.comments,
                new_content: result.content,
                has_bom: content.has_bom,
                style,
            }))
        })
        .collect();

    let mut skipped_generated = 0usize;
    let mut changes: Vec<FileChange> = Vec::new();
    for outcome in outcomes {
        match outcome {
            FileOutcome::Stripped(change) => changes.push(change),
            FileOutcome::Generated => skipped_generated += 1,
            FileOutcome::Unchanged => {}
        }
    }

    let comments: usize = changes.iter().map(|c| c.comments).sum();
    changes.sort_by(|a, b| a.relative.cmp(&b.relative));

    Plan {
        scanned: files.len(),
        skipped_generated,
        changes,
        comments,
    }
}

fn apply(plan: &Plan, args: &StripCommentsArgs, output: &Output) -> (usize, usize) {
    let formatters = resolve_formatters(plan, args, output);
    let mut written = 0usize;
    let mut errors = 0usize;

    for change in &plan.changes {
        let mut content = change.new_content.clone();

        if let Some(formatter) = formatters.get(&change.style) {
            match format_source(change.style, formatter, &change.path, &content) {
                Ok(formatted) => content = formatted,
                Err(error) => {
                    output.warn(&format!("{}: formatter failed ({error})", change.relative));
                    errors += 1;
                    continue;
                }
            }
        }

        let mut bytes = content.into_bytes();
        restore_bom(change.has_bom, &mut bytes);
        match write_file_safe(&change.path, &bytes) {
            Ok(()) => written += 1,
            Err(error) => {
                output.warn(&format!("{}: {error}", change.relative));
                errors += 1;
            }
        }
    }

    (written, errors)
}

/// resolve_formatters picks one formatter per language present in the plan.
///
/// Removing a trailing comment undoes the column alignment a formatter applied,
/// so the formatter is what puts the file back the way its own toolchain writes
/// it. It is optional: a missing gofmt, rustfmt or prettier is reported, not
/// treated as a failure, and --no-format turns the pass off.
fn resolve_formatters(
    plan: &Plan,
    args: &StripCommentsArgs,
    output: &Output,
) -> BTreeMap<CommentStyle, PathBuf> {
    let mut out = BTreeMap::new();
    if args.no_format {
        return out;
    }

    for style in &plan
        .changes
        .iter()
        .map(|c| c.style)
        .collect::<BTreeSet<_>>()
    {
        let found = style
            .formatters()
            .iter()
            .map(PathBuf::from)
            .find(|c| which(c).is_some());
        match found {
            Some(bin) => {
                output.info(&format!(
                    "Formatter: {} via {}",
                    style.label(),
                    bin.display()
                ));
                out.insert(*style, bin);
            }
            None => output.info(&format!(
                "Formatter: none for {} (looked for {})",
                style.label(),
                style.formatters().join(", ")
            )),
        }
    }
    out
}

/// format_source pipes a file through its language formatter on stdin. `go fmt`
/// takes a package rather than stdin, so it is not usable here and only `gofmt`
/// is.
fn format_source(
    style: CommentStyle,
    bin: &Path,
    path: &Path,
    content: &str,
) -> std::result::Result<String, String> {
    let mut cmd = Command::new(bin);
    let mut temp_go: Option<PathBuf> = None;
    match style {
        CommentStyle::Go => {
            // gofmt only reads stdin through a literal `-` argument, and that
            // spelling is Unix-only: the Windows build resolves it as a path
            // and fails with `GetFileAttributesEx -`. Hand it a real file
            // instead, so both platforms format the same way.
            temp_go = Some(spill_go_file(content)?);
            cmd.arg("-w").arg(temp_go.as_ref().expect("just set"));
        }
        CommentStyle::Rust => {
            cmd.args(["--emit", "stdout", "--edition", "2021"]);
        }
        CommentStyle::JavaScript => {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| "unnameable file".to_string())?;
            if bin_name(bin) == "biome" {
                cmd.args(["format", "--stdin-file-path"]).arg(name);
            } else {
                cmd.arg("--stdin-filepath").arg(name);
            }
        }
        CommentStyle::Sql | CommentStyle::GraphQl => {
            return Err("no formatter for this language".to_string());
        }
    }

    let result = pipe_through(&mut cmd, content);

    if let Some(temp) = temp_go {
        // The formatted text now lives in the temp file; the caller wants it
        // back as a string, and the file must not outlive the call either way.
        let formatted = match result {
            Ok(()) => std::fs::read_to_string(&temp).map_err(|e| format!("re-read failed: {e}")),
            Err(e) => Err(e),
        };
        let _ = std::fs::remove_file(&temp);
        return formatted;
    }
    result.map(|()| String::new())
}

/// spill_go_file writes content to a uniquely named `.go` file beside nothing in
/// particular, so gofmt can be pointed at a real path. The name is unique per
/// process and call so parallel workers cannot collide on it.
fn spill_go_file(content: &str) -> std::result::Result<PathBuf, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let dir = std::env::temp_dir();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = format!("refactor-gofmt-{}-{n}.go", std::process::id());
    let path = dir.join(name);
    std::fs::write(&path, content).map_err(|e| format!("temp write failed: {e}"))?;
    Ok(path)
}

fn pipe_through(cmd: &mut std::process::Command, content: &str) -> std::result::Result<(), String> {
    use std::io::Write;

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "no stdin".to_string())?
        .write_all(content.as_bytes())
        .map_err(|e| format!("write failed: {e}"))?;

    let out = child
        .wait_with_output()
        .map_err(|e| format!("wait failed: {e}"))?;
    if !out.status.success() {
        return Err(format!("exited with {}", out.status));
    }
    Ok(())
}

fn bin_name(bin: &Path) -> String {
    bin.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

fn which(candidate: &Path) -> Option<PathBuf> {
    let name = candidate.file_stem()?.to_str()?;
    let path_var = std::env::var_os("PATH")?;
    let exts: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
            .split(';')
            .map(|e| e.to_lowercase())
            .collect()
    } else {
        vec![String::new()]
    };

    for dir in std::env::split_paths(&path_var) {
        for ext in &exts {
            let full = dir.join(format!("{name}{ext}"));
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}

fn show_plan(output: &Output, plan: &Plan, dry_run: bool) {
    if dry_run {
        output.heading("DRY RUN");
    } else {
        output.heading("Plan");
    }

    output.info(&format!("Files scanned: {}", plan.scanned));
    output.info(&format!("Files affected: {}", plan.changes.len()));
    output.info(&format!("Comments: {}", plan.comments));
    output.blank();

    for (label, count) in group_by_language(plan) {
        output.info(&format!("  {label}: {count} file(s)"));
    }
    output.blank();

    for change in &plan.changes {
        output.line(&format!(
            "  {} ({} comments)",
            change.relative, change.comments
        ));
    }

    output.blank();
    if dry_run {
        output.info("No files were changed.");
    }
}

fn group_by_language(plan: &Plan) -> BTreeMap<&'static str, usize> {
    let mut out: BTreeMap<&'static str, usize> = BTreeMap::new();
    for change in &plan.changes {
        *out.entry(change.style.label()).or_insert(0) += 1;
    }
    out
}

fn report(output: &Output, plan: &Plan, dry_run: bool, errors: usize) {
    let data = serde_json::json!({
        "comments": plan.comments,
        "files_scanned": plan.scanned,
        "generated_skipped": plan.skipped_generated,
        "dry_run": dry_run,
    });
    output.print_json_result(&crate::output::ScanResult {
        command: "strip-comments".to_string(),
        files_scanned: plan.scanned,
        files_changed: Some(plan.changes.len()),
        replacements: Some(plan.comments),
        errors: Some(errors),
        skipped: Some(plan.skipped_generated),
        data: Some(data),
    });
}

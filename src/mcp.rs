use crate::cli::{
    CheckArgs, CleanArgs, DuplicatesArgs, FilterArgs, ImportMigrateArgs, ImportsAction,
    ImportsCommand, MigrateArgs, NormalizeArgs, PathsAction, PathsCommand, ReferencesArgs,
    RenameArgs, ReplaceArgs, ScanArgs, UnusedArgs,
};
use crate::commands;
use crate::config;
use crate::context::RepoContext;
use crate::error::{RefactorError, Result};
use crate::exec::ExecOptions;
use crate::output::{Capture, Output};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

const SERVER_INSTRUCTIONS: &str =
    "refactor operates on one repository at a time. Every tool accepts an optional `root` \
     (absolute or relative path; defaults to the server's root). All mutating tools default \
     to a dry run and only write to disk when you pass `apply: true`. Read-only tools (scan, \
     check, references, unused, duplicates) never modify files.";

pub fn run_server(default_root: PathBuf) -> i32 {
    eprintln!(
        "[refactor-mcp] ready (protocol={PROTOCOL_VERSION}, default root={})",
        default_root.display()
    );

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut frame = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                frame.push_str(&line);
                if frame.len() > MAX_FRAME_BYTES {
                    eprintln!("[refactor-mcp] frame exceeded {MAX_FRAME_BYTES} bytes, dropped");
                    frame.clear();
                    continue;
                }
                match serde_json::from_str::<Value>(&frame) {
                    Ok(message) => {
                        frame.clear();
                        if let Some(response) = handle_message(&message, &default_root) {
                            let _ = writeln!(out, "{response}");
                            let _ = out.flush();
                        }
                    }
                    Err(_) => continue,
                }
            }
            Err(_) => break,
        }
    }

    0
}

fn handle_message(message: &Value, default_root: &Path) -> Option<Value> {
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    let id = message.get("id").cloned();

    match method {
        "initialize" => {
            let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
            let requested = params
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL_VERSION);
            id.map(|id| {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": requested,
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "refactor-mcp", "version": env!("CARGO_PKG_VERSION") },
                        "instructions": SERVER_INSTRUCTIONS
                    }
                })
            })
        }
        "notifications/initialized" | "initialized" | "notifications/cancelled" => None,
        "ping" => id.map(|id| json!({"jsonrpc": "2.0", "id": id, "result": {}})),
        "tools/list" => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": tools() }
            })
        }),
        "resources/list" => {
            id.map(|id| json!({"jsonrpc": "2.0", "id": id, "result": { "resources": [] }}))
        }
        "resources/templates/list" => {
            id.map(|id| json!({"jsonrpc": "2.0", "id": id, "result": { "resourceTemplates": [] }}))
        }
        "prompts/list" => {
            id.map(|id| json!({"jsonrpc": "2.0", "id": id, "result": { "prompts": [] }}))
        }
        "tools/call" => {
            let id = id?;
            let name = message["params"]["name"].as_str().unwrap_or("");
            let arguments = &message["params"]["arguments"];
            let arguments = if arguments.is_object() {
                arguments.clone()
            } else {
                json!({})
            };
            let result = match handle_tool_call(name, &arguments, default_root) {
                Ok(result) => result,
                Err(e) => json!({
                    "content": [{ "type": "text", "text": format!("{e}") }],
                    "isError": true
                }),
            };
            Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
        }
        _ => id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": "Method not found" }
            })
        }),
    }
}

fn handle_tool_call(name: &str, arguments: &Value, default_root: &Path) -> Result<Value> {
    match name {
        "scan" => {
            let c = prepare(arguments, default_root, false)?;
            let args = ScanArgs {
                filters: filter_args(arguments),
            };
            let code = commands::scan::run(&c.ctx, &c.output, &args)?;
            Ok(tool_result(c, code))
        }
        "check" => {
            let c = prepare(arguments, default_root, false)?;
            let args = CheckArgs {
                strict: get_bool(arguments, "strict", false),
            };
            let code = commands::check::run(&c.ctx, &c.output, &args)?;
            Ok(tool_result(c, code))
        }
        "replace" => {
            let c = prepare(arguments, default_root, get_bool(arguments, "apply", false))?;
            let args = ReplaceArgs {
                old: req_str(arguments, "old")?,
                new: req_str(arguments, "new")?,
                filters: filter_args(arguments),
                regex: get_bool(arguments, "regex", false),
                case_sensitive: get_bool(arguments, "case_sensitive", false),
                whole_word: get_bool(arguments, "whole_word", false),
                allow_dirty: get_bool(arguments, "allow_dirty", false),
            };
            let code = commands::replace::run(&c.ctx, &c.output, &args, &c.opts)?;
            Ok(tool_result(c, code))
        }
        "rename" => {
            let c = prepare(arguments, default_root, get_bool(arguments, "apply", false))?;
            let args = RenameArgs {
                old: req_path(arguments, "old")?,
                new: req_path(arguments, "new")?,
            };
            let code = commands::rename::run(&c.ctx, &c.output, &args, &c.opts)?;
            Ok(tool_result(c, code))
        }
        "imports" => {
            let c = prepare(arguments, default_root, get_bool(arguments, "apply", false))?;
            let cmd = imports_command(arguments)?;
            let code = commands::imports::run(&c.ctx, &c.output, &cmd, &c.opts)?;
            Ok(tool_result(c, code))
        }
        "paths" => {
            let c = prepare(arguments, default_root, get_bool(arguments, "apply", false))?;
            let cmd = paths_command(arguments)?;
            let code = commands::paths::run(&c.ctx, &c.output, &cmd, &c.opts)?;
            Ok(tool_result(c, code))
        }
        "references" => {
            let c = prepare(arguments, default_root, false)?;
            let args = ReferencesArgs {
                path: req_str(arguments, "path")?,
                filters: filter_args(arguments),
            };
            let code = commands::references::run(&c.ctx, &c.output, &args)?;
            Ok(tool_result(c, code))
        }
        "unused" => {
            let c = prepare(arguments, default_root, false)?;
            let args = UnusedArgs {
                filters: filter_args(arguments),
            };
            let code = commands::unused::run(&c.ctx, &c.output, &args)?;
            Ok(tool_result(c, code))
        }
        "duplicates" => {
            let c = prepare(arguments, default_root, false)?;
            let args = DuplicatesArgs {
                min_size: get_opt_u64(arguments, "min_size"),
                filters: filter_args(arguments),
            };
            let code = commands::duplicates::run(&c.ctx, &c.output, &args)?;
            Ok(tool_result(c, code))
        }
        "normalize" => {
            let c = prepare(arguments, default_root, false)?;
            let args = NormalizeArgs {
                filters: filter_args(arguments),
            };
            let code = commands::normalize::run(&c.ctx, &c.output, &args)?;
            Ok(tool_result(c, code))
        }
        "migrate" => {
            let c = prepare(arguments, default_root, get_bool(arguments, "apply", false))?;
            let plan = PathBuf::from(req_str(arguments, "plan")?);
            let plan = if plan.is_absolute() {
                plan
            } else {
                c.ctx.root.join(plan)
            };
            let args = MigrateArgs { plan };
            let code = commands::migrate::run(&c.ctx, &c.output, &args, &c.opts)?;
            Ok(tool_result(c, code))
        }
        "clean" => {
            let c = prepare(arguments, default_root, get_bool(arguments, "apply", false))?;
            let args = CleanArgs {
                empty_dirs: get_bool(arguments, "empty_dirs", false),
                temp_files: get_bool(arguments, "temp_files", false),
                cache: get_bool(arguments, "cache", false),
            };
            let code = commands::clean::run(&c.ctx, &c.output, &args, &c.opts)?;
            Ok(tool_result(c, code))
        }
        other => Err(RefactorError::Validation(format!("Unknown tool '{other}'"))),
    }
}

fn imports_command(arguments: &Value) -> Result<ImportsCommand> {
    let cmd = match req_str(arguments, "action")?.as_str() {
        "scan" => ImportsCommand {
            action: ImportsAction::Scan(ScanArgs {
                filters: filter_args(arguments),
            }),
        },
        "check" => ImportsCommand {
            action: ImportsAction::Check(CheckArgs {
                strict: get_bool(arguments, "strict", false),
            }),
        },
        "migrate" => ImportsCommand {
            action: ImportsAction::Migrate(ImportMigrateArgs {
                old: req_str(arguments, "old")?,
                new: req_str(arguments, "new")?,
            }),
        },
        "normalize" => ImportsCommand {
            action: ImportsAction::Normalize(NormalizeArgs {
                filters: filter_args(arguments),
            }),
        },
        "unused" => ImportsCommand {
            action: ImportsAction::Unused(UnusedArgs {
                filters: filter_args(arguments),
            }),
        },
        other => {
            return Err(RefactorError::Validation(format!(
                "Unknown imports action '{other}'; expected one of: scan, check, migrate, normalize, unused"
            )));
        }
    };
    Ok(cmd)
}

fn paths_command(arguments: &Value) -> Result<PathsCommand> {
    let cmd = match req_str(arguments, "action")?.as_str() {
        "scan" => PathsCommand {
            action: PathsAction::Scan(ScanArgs {
                filters: filter_args(arguments),
            }),
        },
        "check" => PathsCommand {
            action: PathsAction::Check(CheckArgs {
                strict: get_bool(arguments, "strict", false),
            }),
        },
        "migrate" => PathsCommand {
            action: PathsAction::Migrate(ImportMigrateArgs {
                old: req_str(arguments, "old")?,
                new: req_str(arguments, "new")?,
            }),
        },
        "normalize" => PathsCommand {
            action: PathsAction::Normalize(NormalizeArgs {
                filters: filter_args(arguments),
            }),
        },
        other => {
            return Err(RefactorError::Validation(format!(
                "Unknown paths action '{other}'; expected one of: scan, check, migrate, normalize"
            )));
        }
    };
    Ok(cmd)
}

struct ToolContext {
    ctx: RepoContext,
    output: Output,
    capture: Capture,
    opts: ExecOptions,
}

fn prepare(arguments: &Value, default_root: &Path, apply: bool) -> Result<ToolContext> {
    let root = resolve_root(arguments, default_root)?;
    let mut config = config::load(&root)?;
    if let Some(extensions) = string_array(arguments, "extensions") {
        config.scan.extensions = extensions;
    }
    if let Some(excludes) = string_array(arguments, "exclude") {
        config.scan.exclude.extend(excludes);
    }

    let mut ctx = RepoContext::new(&root, &config)?;
    ctx.init_threads(None);

    let (output, capture) = Output::captured(false, false, false);
    let opts = ExecOptions::new(!apply, apply);

    Ok(ToolContext {
        ctx,
        output,
        capture,
        opts,
    })
}

fn resolve_root(arguments: &Value, default_root: &Path) -> Result<PathBuf> {
    match get_str(arguments, "root") {
        Some(root) => {
            let path = PathBuf::from(root);
            if path.is_absolute() {
                Ok(path)
            } else {
                Ok(std::env::current_dir()?.join(path))
            }
        }
        None => Ok(default_root.to_path_buf()),
    }
}

fn tool_result(context: ToolContext, code: i32) -> Value {
    let text = context.capture.take();
    let content = if text.trim().is_empty() {
        format!("(no output; exit code {code})")
    } else {
        text
    };
    let mut result = json!({
        "content": [{ "type": "text", "text": content }],
        "isError": code >= 2,
    });
    if let Ok(parsed) = serde_json::from_str::<Value>(content.trim()) {
        result["structuredContent"] = parsed;
    }
    result
}

fn req_str(arguments: &Value, key: &str) -> Result<String> {
    get_str(arguments, key)
        .map(str::to_string)
        .ok_or_else(|| RefactorError::Validation(format!("Missing required argument '{key}'")))
}

fn req_path(arguments: &Value, key: &str) -> Result<PathBuf> {
    get_str(arguments, key)
        .map(PathBuf::from)
        .ok_or_else(|| RefactorError::Validation(format!("Missing required argument '{key}'")))
}

fn get_str<'a>(arguments: &'a Value, key: &str) -> Option<&'a str> {
    arguments.get(key).and_then(Value::as_str)
}

fn get_bool(arguments: &Value, key: &str, default: bool) -> bool {
    arguments
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn get_opt_u64(arguments: &Value, key: &str) -> Option<u64> {
    arguments.get(key).and_then(Value::as_u64)
}

fn string_array(arguments: &Value, key: &str) -> Option<Vec<String>> {
    arguments.get(key).and_then(Value::as_array).map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect()
    })
}

fn filter_args(arguments: &Value) -> FilterArgs {
    FilterArgs {
        include: string_array(arguments, "include"),
        exclude: string_array(arguments, "exclude"),
        extensions: string_array(arguments, "extensions"),
    }
}

fn tools() -> Value {
    json!([
        tool_def("scan", "Inventory repository files and directories by extension. Read-only.", schema(&[
            ("root", root_prop()),
        ], &[])),
        tool_def("check", "Validate repository consistency: broken imports, broken references, and duplicate files. Read-only; exit code reflects findings.", schema(&[
            ("root", root_prop()),
            ("strict", bool_prop("Treat warnings as errors")),
        ], &[])),
        tool_def("replace", "Replace a string or regex across source files. Defaults to a dry-run preview; pass apply=true to write changes.", schema(&[
            ("old", string_prop("String or regex pattern to search for (required)")),
            ("new", string_prop("Replacement text; supports regex capture groups such as $1")),
            ("regex", bool_prop("Treat old/new as regex patterns")),
            ("case_sensitive", bool_prop("Case-sensitive matching")),
            ("whole_word", bool_prop("Match whole words only")),
            ("allow_dirty", bool_prop("Skip the git dirty-worktree warning")),
            ("apply", apply_prop()),
            ("root", root_prop()),
            ("include", array_prop("Only process these paths")),
            ("exclude", array_prop("Skip these paths")),
            ("extensions", array_prop("File extensions to process")),
        ], &["old", "new"])),
        tool_def("rename", "Rename a file or directory and rewrite relative import references. Defaults to a dry-run preview; pass apply=true to write changes.", schema(&[
            ("old", string_prop("Existing file/directory path (required)")),
            ("new", string_prop("New file/directory path (required)")),
            ("apply", apply_prop()),
            ("root", root_prop()),
        ], &["old", "new"])),
        tool_def("imports", "Analyze and migrate imports. action=migrate rewrites import paths (apply=true to write); other actions are read-only.", schema(&[
            ("action", enum_prop("Sub-operation", &["scan", "check", "migrate", "normalize", "unused"])),
            ("old", string_prop("Import path to migrate from (required for migrate)")),
            ("new", string_prop("Import path to migrate to (required for migrate)")),
            ("strict", bool_prop("Treat warnings as errors (check action)")),
            ("apply", apply_prop()),
            ("root", root_prop()),
            ("include", array_prop("Only process these paths")),
            ("exclude", array_prop("Skip these paths")),
            ("extensions", array_prop("File extensions to process")),
        ], &["action"])),
        tool_def("paths", "Analyze and migrate path references. action=migrate rewrites paths (apply=true to write); other actions are read-only.", schema(&[
            ("action", enum_prop("Sub-operation", &["scan", "check", "migrate", "normalize"])),
            ("old", string_prop("Path to migrate from (required for migrate)")),
            ("new", string_prop("Path to migrate to (required for migrate)")),
            ("strict", bool_prop("Treat warnings as errors (check action)")),
            ("apply", apply_prop()),
            ("root", root_prop()),
            ("include", array_prop("Only process these paths")),
            ("exclude", array_prop("Skip these paths")),
            ("extensions", array_prop("File extensions to process")),
        ], &["action"])),
        tool_def("references", "Find every file and line that references a given path. Read-only.", schema(&[
            ("path", string_prop("Target path to search references for (required)")),
            ("root", root_prop()),
            ("include", array_prop("Only process these paths")),
            ("exclude", array_prop("Skip these paths")),
            ("extensions", array_prop("File extensions to process")),
        ], &["path"])),
        tool_def("unused", "Detect source files nothing references. Read-only.", schema(&[
            ("root", root_prop()),
            ("include", array_prop("Only process these paths")),
            ("exclude", array_prop("Skip these paths")),
            ("extensions", array_prop("File extensions to process")),
        ], &[])),
        tool_def("duplicates", "Detect files with identical content via blake3 hashing. Read-only.", schema(&[
            ("min_size", number_prop("Minimum file size in bytes to consider")),
            ("root", root_prop()),
            ("include", array_prop("Only process these paths")),
            ("exclude", array_prop("Skip these paths")),
            ("extensions", array_prop("File extensions to process")),
        ], &[])),
        tool_def("normalize", "Report imports/paths that can be normalized (slash style, redundant segments). Read-only.", schema(&[
            ("root", root_prop()),
            ("include", array_prop("Only process these paths")),
            ("exclude", array_prop("Skip these paths")),
            ("extensions", array_prop("File extensions to process")),
        ], &[])),
        tool_def("migrate", "Execute a TOML/JSON migration plan file. Defaults to a dry-run preview; pass apply=true to write changes.", schema(&[
            ("plan", string_prop("Path to the migration plan file (required)")),
            ("apply", apply_prop()),
            ("root", root_prop()),
        ], &["plan"])),
        tool_def("clean", "Remove temporary files, caches, and empty directories. Defaults to a dry-run preview; pass apply=true to write changes.", schema(&[
            ("temp_files", bool_prop("Remove *.tmp, *.bak, .~* temp files")),
            ("cache", bool_prop("Remove cache directories (.cache, __pycache__, .pytest_cache)")),
            ("empty_dirs", bool_prop("Remove empty directories")),
            ("apply", apply_prop()),
            ("root", root_prop()),
        ], &[])),
    ])
}

fn tool_def(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

fn schema(properties: &[(&str, Value)], required: &[&str]) -> Value {
    let mut map = serde_json::Map::with_capacity(properties.len());
    for (key, value) in properties {
        map.insert((*key).to_string(), value.clone());
    }
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": map,
        "required": required,
    })
}

fn string_prop(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

fn bool_prop(description: &str) -> Value {
    json!({ "type": "boolean", "description": description })
}

fn number_prop(description: &str) -> Value {
    json!({ "type": "number", "description": description })
}

fn array_prop(description: &str) -> Value {
    json!({
        "type": "array",
        "items": { "type": "string" },
        "description": description,
    })
}

fn enum_prop(description: &str, values: &[&str]) -> Value {
    json!({
        "type": "string",
        "description": description,
        "enum": values,
    })
}

fn root_prop() -> Value {
    string_prop(
        "Repository root for this operation (absolute or relative; defaults to the server root)",
    )
}

fn apply_prop() -> Value {
    bool_prop("Set to true to apply changes to disk. Defaults to false (dry-run preview).")
}

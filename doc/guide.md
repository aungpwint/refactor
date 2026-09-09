# Refactor — User Guide

**Refactor** is a command-line tool that helps you scan, check, and safely
change large codebases. Think of it as a smart search-and-replace that also
understands imports, paths, and file structure.

---

## What does Refactor do?

| Problem | Command | What happens |
|---------|---------|--------------|
| "How many files are in this repo?" | `scan` | Counts files by type |
| "Are there broken imports?" | `check` | Finds imports that point to missing files |
| "Rename this path everywhere" | `replace` | Finds and replaces text across all files |
| "Move this file" | `rename` | Moves a file and rewrites all imports pointing to it |
| "Which files import this?" | `references` | Shows every file that references a path |
| "What files are never used?" | `unused` | Finds files nobody imports |
| "Are there duplicate files?" | `duplicates` | Finds files with identical content |
| "Migrate from old to new paths" | `migrate` | Runs a batch of planned changes |
| "Update to latest version" | `update` | Downloads and installs the latest release |

---

## Getting started

### 1. Install

**Easiest:** Download a pre-built binary from the
[Releases page](https://github.com/aungpwint/refactor/releases). No
programming knowledge needed.

**For developers:**

```bash
git clone https://github.com/aungpwint/refactor.git
cd refactor
cargo install --path .
```

After installation, the `refactor` command is available everywhere.

### 2. Run your first scan

```bash
refactor --root /path/to/your/project scan
```

You will see output like this:

```
────────────────────────────
Repository
────────────────────────────
  Root: /path/to/your/project

────────────────────────────
Files
────────────────────────────
          ts: 342
         tsx: 128
          js: 57
         php: 201
         json: 89
       Total: 817

────────────────────────────
Directories
────────────────────────────
       Total: 124
       Source: 817
```

### 3. Check for problems

```bash
refactor --root /path/to/your/project check
```

This finds broken imports, broken references, and duplicate files.

---

## The two types of commands

Understanding this distinction keeps your code safe:

### Read-only commands (never change files)

These commands only **look** at your code. You can run them freely:

- `scan` — count and categorize files
- `check` — find broken imports, references, duplicates
- `references` — find files that reference a path
- `unused` — find files nobody imports
- `duplicates` — find identical files
- `imports scan` — list all imports in the codebase
- `imports check` — find broken or inconsistent imports
- `paths scan` — find broken path references
- `paths check` — validate all path references

### Mutating commands (change files)

These commands **modify** your files. They always require one of:

- `--dry-run` — preview what would change, then stop (safest)
- `--yes` — skip the confirmation prompt and apply changes

```bash
# SAFE: just show me what would happen
refactor replace --dry-run "old/path" "new/path"

# APPLIES CHANGES: rewrite files
refactor replace --yes "old/path" "new/path"
```

---

## Global options

These options work with **every** command. Put them before the command name.

| Option | What it does | Example |
|--------|-------------|---------|
| `--root <PATH>` | Which repo to work on (default: current directory) | `--root /src/my-app` |
| `--dry-run` | Preview changes without writing anything | `--dry-run` |
| `-y, --yes` | Skip confirmation prompts | `--yes` |
| `-v, --verbose` | Show extra details | `--verbose` |
| `-q, --quiet` | Show only errors | `--quiet` |
| `--json` | Output machine-readable JSON | `--json` |
| `--no-color` | Turn off colored output | `--no-color` |
| `--extensions <LIST>` | Only process these file types | `--extensions ts,tsx` |
| `--exclude <LIST>` | Skip these directories | `--exclude vendor,node_modules` |
| `--include <LIST>` | Only look in these directories | `--include src,app` |
| `--threads <N>` | How many parallel threads to use | `--threads 8` |

### Order matters

Global options go **before** the command:

```bash
refactor --root /my/repo --dry-run replace "old" "new"
#            ^^^^^^^^^^^  ^^^^^^^^
#            global opts   command
```

---

## Configuration file

Create a `.refactor.toml` file in your project root to set defaults:

```toml
[scan]
# Which file types to scan (default: ts, tsx, js, jsx, php, json, css, scss, md, vue, svelte)
extensions = ["ts", "tsx", "js", "jsx", "php", "json"]

# Which directories to skip (default includes .git, node_modules, vendor, dist, build, etc.)
exclude = [".git", "node_modules", "vendor", "dist", "build"]

[typescript]
# Path to tsconfig.json (auto-detected if omitted)
tsconfig = "tsconfig.app.json"

[output]
# Force color on/off (auto-detected if omitted)
color = true
```

You do **not** need a config file. The defaults work for most projects.

CLI flags always override the config file.

---

## How safety works

Refactor is built to prevent accidental damage:

### 1. Dry run is always available

Every mutating command supports `--dry-run`. Nothing is changed — you just
see what **would** happen:

```bash
refactor replace --dry-run "admin/Button" "components/Button"
```

Output:

```
────────────────────────────
Replace
────────────────────────────
  Old: "admin/Button"
  New: "components/Button"

  Plan: 12 files would be changed, 23 replacements

  admin/Header.tsx:5   → import { Button } from "../admin/Button"
  admin/Sidebar.tsx:12 → import { Button } from "../admin/Button"
  ... (10 more files)
```

### 2. Confirmation is required

Mutating commands refuse to run without `--yes` or `--dry-run`:

```bash
refactor rename "src/old.ts" "src/new.ts"
# Output: Rename requires --yes to confirm (or use --dry-run)
```

### 3. Git dirty worktree warning

If you have uncommitted changes, mutating commands warn you:

```
! Git working tree is dirty — modified/untracked files may be overwritten.
  Use --allow-dirty to silence this warning.
```

### 4. Binary files are skipped

Images, videos, fonts, archives, and other binary files are automatically
detected and skipped. Your binaries are never touched.

### 5. Line endings and BOM are preserved

If your files use CRLF (Windows) or have a UTF-8 BOM, those are kept intact
when the tool rewrites files.

### 6. Atomic writes

Files are written to a temporary location first, then moved into place. This
prevents partial corruption if something goes wrong mid-write.

---

## Exit codes

When a command finishes, it returns an exit code. Use this in scripts:

| Code | Meaning | What to do |
|------|---------|------------|
| `0` | Success | Everything worked |
| `1` | Problems found | `check` found issues, or validation failed |
| `2` | Usage error | Missing `--yes`, or path doesn't exist |
| `3` | Config error | Bad `.refactor.toml` or migration plan |
| `4` | Filesystem error | Can't read/write files (permissions?) |
| `5` | Conflict | Rename destination already exists |
| `6` | Partial failure | Some files updated, some failed |

### Using in CI

```bash
# Fail the build if check finds issues
refactor check --strict
if [ $? -ne 0 ]; then
  echo "Repository has consistency issues!"
  exit 1
fi
```

---

## Architecture (for developers)

If you want to understand or contribute to the codebase:

```
src/
├── main.rs              Entry point — parses CLI, loads config, runs command
├── cli.rs               All command-line argument definitions (clap)
├── config.rs            Reads .refactor.toml, provides defaults
├── context.rs           RepoContext — holds repo root, config, git state
├── error.rs             Error types mapped to exit codes
├── output.rs            Handles colored, JSON, and quiet output
├── commands/            One file per command (scan.rs, check.rs, etc.)
├── scanner/             Walks directories, scans file contents
├── analyzers/           Finds problems (broken imports, duplicates, etc.)
├── refactor/            Makes changes (replace, rename, migrate)
├── filesystem/          Reads and writes files safely
├── languages/           Understands TS, JS, PHP, JSON syntax
└── utils/               Hashing and path helpers
```

**How a command runs:**

1. CLI arguments are parsed (`cli.rs`)
2. Config is loaded from `.refactor.toml` (`config.rs`)
3. A `RepoContext` is created with the repo root and git state (`context.rs`)
4. The matching command function is called (`commands/*.rs`)
5. The command uses scanners and analyzers to gather data
6. If it is a mutating command, the refactor engine applies changes
7. Results are printed via the output formatter (`output.rs`)

---

## Learn more

- [Command Reference](commands.md) — every command, every flag, with examples
- [Real-World Examples](examples.md) — step-by-step workflows for common tasks

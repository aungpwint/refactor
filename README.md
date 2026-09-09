# refactor

A production-grade, cross-platform Rust CLI for safely **scanning, analyzing,
refactoring, renaming, validating, and migrating** large software codebases.
It targets the TBD monorepo (Laravel + React/TypeScript) but works on any
repository.

> **New to refactor?** Read the documentation:
>
> - [Installation](doc/install.md) — how to install on any device
> - [User Guide](doc/guide.md) — how it works, safety features, configuration
> - [Command Reference](doc/commands.md) — every command, every flag, with output examples
> - [Real-World Examples](doc/examples.md) — step-by-step workflows for common tasks

## Features

- **scan** — File and directory inventory broken down by extension.
- **check** — Validate repository consistency: broken imports, broken
  references, and duplicate files. Returns a non-zero exit code on findings.
- **replace** — Replace strings or regex patterns across source files.
- **rename** — Rename files/directories and rewrite relative import references.
- **imports** — Scan, check, migrate, normalize, and detect unused imports.
- **paths** — Scan, check, and migrate path references.
- **references** — Find every file that references a given path.
- **unused** — Detect unreferenced source files.
- **duplicates** — Detect files with identical content (blake3 hashing).
- **normalize** — Normalize import/path styles.
- **migrate** — Execute a TOML/JSON migration plan.
- **clean** — Remove temp files, cache files, and empty directories.

## Safety by default

- **Dry run** (`--dry-run`): prints the full plan and touches nothing.
- **`--yes`**: required by `rename` to confirm before making changes.
- **Read-only analysis** commands (`scan`, `check`, `references`, `unused`,
  `duplicates`) never modify files.
- **BOM preserved**: UTF-8 BOM is retained when rewriting files.
- **Line endings preserved**: CRLF/LF are left intact.
- **Binary-safe**: binary files (by extension or null-byte sniff) are skipped.
- **Git-aware**: respects `.gitignore` during traversal and warns on a dirty
  worktree before mutating commands.
- **Atomic writes**: files are written via a temp-file + rename to avoid
  partial writes.

## Install

```bash
cargo install --path .
```

## Usage

```bash
refactor --root /path/to/repo scan
refactor --root /path/to/repo check --strict
refactor --root /path/to/repo replace --dry-run "old/pattern" "new/pattern"
refactor --root /path/to/repo replace --yes "admin/resource-kit" "components/resource-kit"
refactor --root /path/to/repo rename --yes "src/old.ts" "src/new.ts"
refactor --root /path/to/repo --json scan
```

### Global options

```
--root <PATH>       Repository root (default: .)
-v, --verbose       Verbose output
-q, --quiet         Suppress non-essential output
--json              Machine-readable JSON output
--no-color          Disable colored terminal output
--dry-run           Show what would change without modifying anything
-y, --yes           Skip confirmation prompts
--include <PATHS>   Include only these paths (comma-separated)
--exclude <PATHS>   Exclude these paths (comma-separated)
--extensions <EXT>  File extensions to process (comma-separated)
--threads <N>       Number of parallel threads
```

### Configuration

Optional `.refactor.toml` in the repository root:

```toml
[scan]
extensions = ["ts", "tsx", "js", "jsx", "php", "json", "css", "scss"]
exclude = [".git", "node_modules", "vendor", "dist", "build"]
```

### Migration plans

```toml
version = 1

[[operations]]
type = "replace"
from = "legacy/package"
to = "modern/package"
```

```bash
refactor --root . --yes migrate plan.toml
```

## Exit codes

| Code | Meaning |
|------|---------|
| 0    | Success |
| 1    | Findings (check failed, validation error) |
| 2    | Usage / confirmation required / path safety |
| 3    | Configuration / plan error |
| 4    | Filesystem error |
| 5    | Conflict (e.g., rename destination exists) |
| 6    | (reserved) partial failure |

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

## License

MIT
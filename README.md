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
- **strip-comments** — Remove comments from Go, Rust, JS/TS/JSX/TSX, SQL, and
  GraphQL sources with a per-language lexer, so `//`, `/* */`, `--`, and `#`
  are only removed outside strings, raw strings, regex literals, and
  dollar-quoted bodies.
- **mcp** — Run all of the above as a Model Context Protocol server over
  stdio, so AI agents (opencode, Claude Code, …) can call every command as a
  tool. See [the MCP command reference](doc/commands.md#mcp--model-context-protocol-server).

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
- **Directives are not comments**: `strip-comments` leaves functional markers
  alone by default — `//go:build`, `//go:embed`, `//go:generate`, `//nolint`,
  `// rustfmt::skip`, `/// <reference>`, `// @ts-ignore`, `eslint-disable`,
  `prettier-ignore`, source-map pragmas, and SQL migration markers such as
  `-- +migrate down`. Pass `--strip-directives` to remove those too.
- **Generated files are skipped** unless you pass `--generated`.

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
refactor --root /path/to/repo strip-comments --dry-run
refactor --root /path/to/repo strip-comments --lang ts,tsx
refactor --root /path/to/repo strip-comments --lang sql,graphql --allow-dirty
refactor mcp --root /path/to/repo
```

### strip-comments

Removes comments from source files, narrowing the work with `--lang`:

| Value | Files |
|-------|-------|
| `go` | `.go` |
| `rust` | `.rs` |
| `js`, `jsx` | `.js`, `.jsx`, `.mjs`, `.cjs` |
| `ts`, `tsx` | `.ts`, `.tsx`, `.mts`, `.cts` |
| `react`, `web` | the whole JS/TS family |
| `sql` | `.sql` |
| `graphql` | `.graphql`, `.gql`, `.graphqls` |
| `all` (default) | every language above |

Each language gets its own lexer rather than a regex, so comment-looking text
inside a string, raw string, regex literal, or SQL dollar-quoted body survives:

```sql
-- this goes
SELECT 'https://example.com' AS url, $$ -- this is a body comment, not the file's $$ AS q;
```

GraphQL `"description"` blocks are schema rather than commentary and are always
kept. Output is piped through the language formatter when one is available
(`gofmt`, `rustfmt`, `prettier`, then `biome`); a missing formatter is reported
but is not an error, and `--no-format` turns formatting off.

Run it twice and the second pass reports nothing left to strip, so it is safe to
re-run over an already-processed tree.

### AI integration (MCP)

`refactor mcp` serves every command as an MCP tool over stdio. Each tool
accepts an optional `root`; mutating tools (`replace`, `rename`, `imports`
migrate, `paths` migrate, `migrate`, `clean`, `strip-comments`) default to a
safe dry run and write only when you pass `apply: true`.

opencode config:

```jsonc
{
  "mcp": {
    "refactor": { "type": "local", "command": ["refactor", "mcp", "--root", "/path/to/repo"], "enabled": true }
  }
}
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
# Command Reference

Every command in Refactor, explained with flags, examples, and output.

---

## `scan`

**What it does:** Counts all files in your repository, grouped by file type.

**Type:** Read-only (never changes files)

```bash
refactor scan
refactor scan --extensions ts,tsx,js
refactor scan --exclude node_modules,vendor
refactor --json scan
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--extensions <LIST>` | Only count these file types |
| `--exclude <LIST>` | Skip these directories |
| `--include <LIST>` | Only look in these directories |

**Example output:**

```
────────────────────────────
Repository
────────────────────────────
  Root: /home/user/my-app

────────────────────────────
Files
────────────────────────────
          ts: 142
         tsx: 67
          js: 34
          php: 89
        json: 23
          css: 18
       Total: 373

────────────────────────────
Directories
────────────────────────────
       Total: 56
       Source: 373
```

---

## `check`

**What it does:** Scans your entire repo for three kinds of problems:
1. **Broken imports** — files that import something that does not exist
2. **Broken references** — paths that point to deleted or moved files
3. **Duplicate files** — files with identical content

**Type:** Read-only

```bash
refactor check
refactor check --strict
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--strict` | Treat warnings as errors (exit code 1 if any warnings) |

**Example output:**

```
────────────────────────────
Checking repository consistency
────────────────────────────

────────────────────────────
Results
────────────────────────────
  ! 3 import issues found
    src/utils/helpers.ts:5 — @old/legacy-utils (module not found)
    src/pages/Dashboard.tsx:12 — @admin/old-button (module not found)
    src/components/Header.tsx:3 — ../deprecated/util (file not found)

────────────────────────────

  ✓ No broken references found

────────────────────────────

  ! 1 duplicate groups found
    Hash: a1b2c3d4e5f6
      src/utils/old-helper.ts
      src/utils/new-helper.ts

  Errors: 1, Warnings: 3
```

**Exit code:** `0` if clean, `1` if issues found (or warnings with `--strict`).

---

## `replace`

**What it does:** Finds a text pattern across all source files and replaces it
with new text.

**Type:** Mutating (changes files)

```bash
refactor replace "old/path" "new/path"
refactor replace --dry-run "old/path" "new/path"
refactor replace --yes "old/path" "new/path"
refactor replace --regex "import\s+\{.*\}" "import { foo }"
refactor replace --case-sensitive "Button" "Btn"
refactor replace --whole-word "admin" "components"
refactor replace --allow-dirty "old" "new"
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--regex` | Treat the search pattern as a regular expression |
| `--case-sensitive` | Match exact case (default: case-insensitive) |
| `--whole-word` | Only match when the pattern is a complete word |
| `--dry-run` | Preview changes without writing |
| `--yes` | Apply changes without confirmation |
| `--allow-dirty` | Skip the dirty worktree warning |

**What happens:**

1. Scans all source files matching your `--extensions` filter
2. Finds every occurrence of the old pattern
3. Shows a plan: which files, which lines, what changes
4. If `--dry-run`: stops here, nothing changed
5. If not `--yes`: asks for confirmation
6. Rewrites each file (preserving line endings and BOM)

**Example output (dry run):**

```
────────────────────────────
Replace
────────────────────────────
  Old: "admin/Button"
  New: "components/Button"

────────────────────────────
  Plan: 5 files affected, 8 replacements

  src/pages/Dashboard.tsx:12  import { Button } from "../admin/Button"
  src/pages/Dashboard.tsx:45  <Button onClick={handleClick}>
  src/components/Layout.tsx:3 import { Button } from "../admin/Button"
  src/components/Layout.tsx:19 <Button variant="primary">
  src/components/Header.tsx:7 import { Button } from "../../admin/Button"
```

---

## `rename`

**What it does:** Renames a file or directory, then finds and rewrites every
import or reference that pointed to the old name.

**Type:** Mutating (changes files)

```bash
refactor rename "src/old.ts" "src/new.ts"
refactor rename --dry-run "src/old.ts" "src/new.ts"
refactor rename --yes "packages/foo" "packages/bar"
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Preview what would happen without moving anything |
| `--yes` | Rename without confirmation |

**What happens:**

1. Plans the rename — checks the destination does not already exist
2. Scans all source files for imports/references to the old path
3. Shows which files need their imports updated
4. If `--dry-run`: stops here
5. Moves the file/directory
6. Rewrites all import paths in referencing files

**Example output:**

```
────────────────────────────
Rename
────────────────────────────
  Source: src/components/old-button.tsx
  Dest:   src/components/new-button.tsx

  Files to update: 4

  src/pages/Home.tsx:3      import { Button } from "../components/old-button"
  src/pages/About.tsx:5     import { Button } from "../components/old-button"
  src/components/Modal.tsx:2 import { Button } from "./old-button"
  src/App.tsx:8             import { Button } from "./components/old-button"
```

---

## `imports`

A group of subcommands for working with import statements.

### `imports scan`

Lists all imports found in the codebase.

```bash
refactor imports scan
refactor imports scan --extensions ts,tsx
```

**Example output:**

```
────────────────────────────
Import Scan
────────────────────────────
  Files with imports: 89
  Total imports: 342
  Total exports: 127

  Sample imports:
    src/pages/Home.tsx:1 [static] — react
    src/pages/Home.tsx:2 [static] — ./components/Header
    src/pages/Home.tsx:3 [static] — ../utils/api
```

### `imports check`

Finds broken or inconsistent imports.

```bash
refactor imports check
```

**Checks for:**
- Imports pointing to files that do not exist
- Mixed slash styles (`/` vs `\`)
- Double slashes (`//`)
- Redundant path segments (`./../`)

### `imports migrate`

Rewrites import paths matching a pattern.

```bash
refactor imports migrate "old/package" "new/package"
refactor imports migrate --dry-run "old/package" "new/package"
```

**Example:**

```bash
refactor imports migrate "@legacy/utils" "@modern/utils"
```

This finds every file that imports from `@legacy/utils` and rewrites it to
`@modern/utils`.

### `imports normalize`

Reports imports that could be cleaned up (inconsistent slashes, double slashes,
redundant segments).

```bash
refactor imports normalize
```

### `imports unused`

Finds imports that appear to be unused.

```bash
refactor imports unused
```

---

## `paths`

A group of subcommands for working with file path references (non-import paths
like configuration paths, route definitions, etc.).

### `paths scan`

Finds broken path references in the codebase.

```bash
refactor paths scan
```

### `paths check`

Validates that all path references point to existing files.

```bash
refactor paths check
```

### `paths migrate`

Rewrites path references matching a pattern.

```bash
refactor paths migrate "old/directory" "new/directory"
refactor paths migrate --dry-run "old/directory" "new/directory"
```

### `paths normalize`

Reports paths that could be cleaned up.

```bash
refactor paths normalize
```

---

## `references`

**What it does:** Finds every file that mentions a given path string.

**Type:** Read-only

```bash
refactor references "src/components/Button"
refactor references --extensions ts,tsx "shared/utils"
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--extensions <LIST>` | Only search in these file types |

**Why use this:** Before renaming or moving a file, run this to see the full
"blast radius" — every file that will break if you move it.

**Example output:**

```
────────────────────────────
References
────────────────────────────
  Target: src/components/Button

  Found 6 references:

  1. src/pages/Home.tsx
     line 1 [import]: import { Button } from "../components/Button"
     line 24 [path]: <Button variant="primary">
  2. src/pages/Dashboard.tsx
     line 3 [import]: import { Button } from "../components/Button"
  3. src/components/Layout.tsx
     line 5 [import]: import { Button } from "./Button"
  4. src/App.tsx
     line 2 [import]: import { Button } from "./components/Button"
  5. src/config/routes.ts
     line 8 [path]: component: "src/components/Button"
  6. README.md
     line 42 [path]: See src/components/Button for details
```

---

## `unused`

**What it does:** Finds source files that are never imported or required by
any other file.

**Type:** Read-only

```bash
refactor unused
refactor unused --extensions ts,tsx
refactor unused --exclude test,mock
```

**Example output:**

```
────────────────────────────
Unused Files
────────────────────────────
  ! 3 potentially unused files:
    src/utils/old-helpers.ts
    src/components/DeprecatedWidget.tsx
    src/pages/ArchivedPage.tsx
```

---

## `duplicates`

**What it does:** Finds groups of files that have exactly the same content,
using blake3 cryptographic hashing.

**Type:** Read-only

```bash
refactor duplicates
refactor duplicates --min-size 1024
refactor duplicates --extensions ts,tsx
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--min-size <BYTES>` | Ignore files smaller than this (skip tiny files) |

**Example output:**

```
────────────────────────────
Duplicate Files
────────────────────────────
  ! 2 duplicate groups:

  Hash: 7f83b1657ff1fc53b92...
    src/utils/old-helper.ts
    src/utils/backup/helper.ts

  Hash: a3f2c8d9e1b4...
    src/components/Button.tsx
    src/components/old/Button.tsx
```

---

## `normalize`

**What it does:** Reports import and path styles that are inconsistent and
could be cleaned up (mixed slashes, double slashes, redundant segments).

**Type:** Read-only

```bash
refactor normalize
refactor normalize --extensions ts,tsx
```

**Example output:**

```
────────────────────────────
Normalize
────────────────────────────
  5 items could be normalized
```

Use `--verbose` to see which specific items need normalization.

---

## `migrate`

**What it does:** Runs a batch of planned changes from a TOML or JSON file.

**Type:** Mutating (changes files)

```bash
refactor migrate plan.toml
refactor migrate --dry-run plan.toml
refactor migrate --yes plan.json
```

**What happens:**

1. Loads and parses the plan file
2. Validates the plan against your codebase
3. Shows all operations that will be performed
4. If `--dry-run`: stops here
5. Executes each operation in order

See the [Migration Plans section in the guide](guide.md#migration-plans)
for the plan file format.

**Example output (dry run):**

```
────────────────────────────
Migration
────────────────────────────
  Plan: migration.toml
  Operations: 3

  ✓ Plan is valid

  Dry run — showing planned operations:

  [1/3] replace: "legacy/utils" → "modern/utils"
  [2/3] replace: "old/components" → "new/components"
  [3/3] import:  "@admin/button" → "@components/button"

  Dry run complete — no changes were made.
```

---

## `clean`

**What it does:** Removes temporary files, cache directories, and empty
directories from your project.

**Type:** Mutating (deletes files)

```bash
refactor clean --temp-files
refactor clean --cache
refactor clean --empty-dirs
refactor clean --temp-files --cache --empty-dirs
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--temp-files` | Remove `.tmp`, `.bak`, and `.~*` files |
| `--cache` | Remove `.cache`, `__pycache__`, `.pytest_cache` directories |
| `--empty-dirs` | Remove directories that contain no files |

**Important:** You must specify at least one flag. Running `refactor clean`
with no flags tells you what flags are available.

**Example output:**

```
────────────────────────────
Clean
────────────────────────────
  Scanning for temporary files...
  Removed 4 temporary files
  Scanning for cache files...
  Removed 2 cache files
  Scanning for empty directories...
  Removed 3 empty directories

  ✓ Cleaned 9 items
```

# Real-World Examples

Step-by-step workflows for common tasks you will actually do.

---

## Example 1: Understand a new codebase

You just joined a project and need to understand its structure.

```bash
# Step 1: See what types of files exist and how many
refactor --root /path/to/project scan

# Step 2: Check for existing problems
refactor --root /path/to/project check

# Step 3: Find duplicate files (may indicate copy-paste code)
refactor --root /path/to/project duplicates --min-size 1024

# Step 4: Find files nobody uses
refactor --root /path/to/project unused --extensions ts,tsx
```

---

## Example 2: Safely rename a component

You want to rename `Button.tsx` to `PrimaryButton.tsx` and update every
file that imports it.

```bash
# Step 1: See which files reference Button.tsx (dry run is automatic for references)
refactor references "components/Button"

# Step 2: Preview the rename — nothing changes yet
refactor rename --dry-run "src/components/Button.tsx" "src/components/PrimaryButton.tsx"

# Step 3: If the preview looks good, apply it
refactor rename --yes "src/components/Button.tsx" "src/components/PrimaryButton.tsx"
```

---

## Example 3: Migrate import paths after restructuring

You moved code from `src/admin/` to `src/components/` and need to update
every import.

```bash
# Step 1: Preview the replacement
refactor replace --dry-run "admin/" "components/"

# Step 2: If the preview shows the right files, apply it
refactor replace --yes "admin/" "components/"

# Step 3: Verify nothing is broken
refactor check
```

---

## Example 4: Use a migration plan for a large refactor

You are doing a big restructuring that involves multiple steps. Write a plan
file and execute it.

Create `migration.toml`:

```toml
version = 1

[[operations]]
type = "replace"
from = "legacy/api"
to = "modern/api"

[[operations]]
type = "replace"
from = "old/utils"
to = "new/utils"

[[operations]]
type = "import"
from = "@legacy/auth"
to = "@modern/auth"

[[operations]]
type = "rename"
from = "src/pages/old-dashboard"
to = "src/pages/dashboard"
```

Now run it:

```bash
# Step 1: Preview everything the plan will do
refactor --dry-run migrate migration.toml

# Step 2: Apply the plan
refactor --yes migrate migration.toml

# Step 3: Verify the codebase is still healthy
refactor check --strict
```

---

## Example 5: CI pipeline integration

Add Refactor checks to your GitHub Actions or CI pipeline.

```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  refactor-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Refactor
        run: cargo install --path ./refactor

      - name: Check repository consistency
        run: refactor check --strict --json
```

The `--strict` flag makes the command exit with code 1 if any warnings exist,
which fails the CI step.

---

## Example 6: Find and remove dead code

You suspect there are unused files in your project.

```bash
# Step 1: Find files that nothing imports
refactor unused --extensions ts,tsx

# Step 2: For each result, check if anything actually references it
refactor references "src/utils/old-helper"

# Step 3: If the file has zero references, you can safely delete it
# (Refactor does not delete files — use your editor or git for that)
rm src/utils/old-helper.ts

# Step 4: Also check for unused imports inside files
refactor imports unused
```

---

## Example 7: Normalize messy import paths

Your codebase has inconsistent import styles — some use `../`, some use
`@/`, some have double slashes.

```bash
# Step 1: See what needs normalizing
refactor imports normalize

# Step 2: Check for broader path issues
refactor paths check

# Step 3: Fix specific broken paths
refactor imports migrate "old/path" "new/path"

# Step 4: Verify everything is clean
refactor check --strict
```

---

## Example 8: Clean up after a big refactor

You just finished renaming and moving lots of files. Time to clean up.

```bash
# Step 1: Remove temporary files created during editing
refactor clean --temp-files

# Step 2: Remove cache directories
refactor clean --cache

# Step 3: Remove empty directories left behind
refactor clean --empty-dirs

# Step 4: Verify the repo is healthy
refactor check
```

---

## Example 9: Batch replace with regex

You need to update a pattern across many files, but only when it matches
specific criteria.

```bash
# Replace all imports that use the old naming convention
refactor replace --regex --yes "import\s+\{\s*old_(\w+)\s*\}" "import { new_$1 }"

# Replace only whole words (avoid partial matches)
refactor replace --whole-word --yes "admin" "components"

# Case-sensitive replacement
refactor replace --case-sensitive --yes "AdminButton" "PrimaryButton"
```

---

## Example 10: JSON output for scripting

Use `--json` to get machine-readable output you can parse in scripts.

```bash
# Get scan results as JSON
refactor --json scan > scan-results.json

# Use jq to extract specific data
refactor --json scan | jq '.data.total_files'

# Pipe check results into a reporting tool
refactor --json check | jq '.data.import_issues'
```

---

## Quick reference

| Task | Command |
|------|---------|
| See repo structure | `refactor scan` |
| Find problems | `refactor check` |
| Find problems (strict) | `refactor check --strict` |
| Preview a replacement | `refactor replace --dry-run "old" "new"` |
| Apply a replacement | `refactor replace --yes "old" "new"` |
| Preview a rename | `refactor rename --dry-run "old" "new"` |
| Apply a rename | `refactor rename --yes "old" "new"` |
| See who uses a file | `refactor references "path/to/file"` |
| Find dead files | `refactor unused` |
| Find duplicates | `refactor duplicates` |
| Migrate imports | `refactor imports migrate "old" "new"` |
| Check imports | `refactor imports check` |
| Clean up | `refactor clean --temp-files --cache --empty-dirs` |
| Run a plan | `refactor migrate plan.toml` |

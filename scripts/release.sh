#!/usr/bin/env bash
#
# release.sh — Developer workflow for cutting a release.
#
# Safe by default: never pushes anything without explicit confirmation.
#
# Usage:
#   ./scripts/release.sh                     # release current version
#   ./scripts/release.sh --next 1.0.0        # bump version, then release
#   ./scripts/release.sh --push              # also push the tag (confirms)
#   ./scripts/release.sh --yes               # skip interactive confirmations
#
# Steps:
#   1. Working tree must be clean (unless --allow-dirty).
#   2. Version in Cargo.toml matches expected version.
#   3. Runs quality gates: fmt, clippy, tests.
#   4. Creates the annotated tag v<version>.
#   5. Pushes the tag (only with --push, after confirmation).
#
# The tag push triggers .github/workflows/release.yml.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# --- Argument parsing ---
NEXT=""
PUSH=false
YES=false
ALLOW_DIRTY=false
SKIP_CHECKS=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --next)
      NEXT="$2"
      shift 2
      ;;
    --push)
      PUSH=true
      shift
      ;;
    --yes|-y)
      YES=true
      shift
      ;;
    --allow-dirty)
      ALLOW_DIRRY=true
      shift
      ;;
    --skip-checks)
      SKIP_CHECKS=true
      shift
      ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Usage: $0 [--next VERSION] [--push] [--yes] [--allow-dirty] [--skip-checks]" >&2
      exit 1
      ;;
  esac
done

# --- Helpers ---
fail() {
  echo -e "\nERROR: $1\n" >&2
  exit 1
}

confirm() {
  if $YES; then return 0; fi
  read -r -p "$1 [y/N] " answer
  [[ "$answer" =~ ^[Yy]([Ee][Ss])?$ ]]
}

# --- Preflight ---
cd "$ROOT_DIR"

# Check we're in a git repo
git rev-parse --git-dir >/dev/null 2>&1 || fail "Not inside a Git repository."

# Check clean tree
if ! $ALLOW_DIRRY; then
  STATUS=$(git status --porcelain)
  if [ -n "$STATUS" ]; then
    fail "Working tree is not clean. Commit or stash first, or re-run with --allow-dirty."
  fi
fi

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

# Bump version if --next provided
if [ -n "$NEXT" ]; then
  if ! [[ "$NEXT" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
    fail "Invalid --next version '$NEXT'. Use MAJOR.MINOR.PATCH (e.g. 1.0.0)"
  fi
  echo "[release] Bumping version to $NEXT..."
  sed -i "s/^version = \".*\"/version = \"$NEXT\"/" Cargo.toml
  CURRENT_VERSION="$NEXT"
fi

TAG="v${CURRENT_VERSION}"

# Check tag doesn't already exist
if git tag -l "$TAG" | grep -q "$TAG"; then
  fail "Tag $TAG already exists. Delete it first if this is intentional."
fi

echo -e "\n[release] Releasing Refactor $TAG\n"

# --- Quality gates ---
if ! $SKIP_CHECKS; then
  echo "[release] cargo fmt --check ..."
  cargo fmt --check || fail "Format check failed. Run 'cargo fmt' first."

  echo "[release] cargo clippy ..."
  cargo clippy --all-targets -- -D warnings || fail "Clippy check failed."

  echo "[release] cargo test ..."
  cargo test || fail "Tests failed."
else
  echo "[release] --skip-checks: quality gates skipped"
fi

# --- Create tag ---
if ! confirm "Create annotated tag $TAG?"; then
  echo "[release] Aborted — no tag created."
  exit 0
fi

git tag -a "$TAG" -m "Refactor $TAG"
echo "[release] Created tag $TAG"

# --- Push ---
if $PUSH; then
  if confirm "Push $TAG to origin (triggers the release workflow)?"; then
    git push origin "$TAG"
    echo -e "\n[release] Pushed $TAG. Watch the release at:"
    echo "  https://github.com/aungpwint/refactor/actions"
  else
    echo "[release] Not pushed. The tag is local only."
  fi
else
  echo -e "\n[release] Done. Push the tag when ready:\n"
  echo "  git push origin $TAG\n"
  echo "This triggers .github/workflows/release.yml."
fi

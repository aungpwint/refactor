#!/usr/bin/env bash
#
# validate-release.sh — Pre-publish gate for release artifacts.
#
# Usage:
#   ./scripts/validate-release.sh <directory> <version>
#
# Checks:
#   1. At least one binary artifact exists.
#   2. No artifact is zero bytes.
#   3. All filenames contain 'refactor-'.
#   4. checksums.txt exists and covers every binary.
#   5. All checksums match.

set -euo pipefail

if [ $# -lt 2 ]; then
  echo "Usage: $0 <directory> <version>" >&2
  exit 1
fi

DIR="$1"
VERSION="$2"

if [ ! -d "$DIR" ]; then
  echo "Error: Directory '$DIR' does not exist" >&2
  exit 1
fi

cd "$DIR"
ERRORS=0

echo "=== Validating release artifacts ==="
echo "Directory: $DIR"
echo "Version:   $VERSION"
echo ""

# 1. Check at least one binary exists
BINARY_COUNT=$(ls -1 refactor-* 2>/dev/null | wc -l)
if [ "$BINARY_COUNT" -lt 1 ]; then
  echo "FAIL: No binary artifacts found (expected files matching refactor-*)"
  ERRORS=$((ERRORS + 1))
else
  echo "OK: Found $BINARY_COUNT binary artifacts"
fi

# 2. Check no zero-byte files
for f in refactor-*; do
  if [ -f "$f" ] && [ ! -s "$f" ]; then
    echo "FAIL: Artifact '$f' is zero bytes"
    ERRORS=$((ERRORS + 1))
  fi
done

# 3. Check checksums.txt exists
if [ ! -f "checksums.txt" ]; then
  echo "FAIL: checksums.txt is missing"
  ERRORS=$((ERRORS + 1))
elif [ ! -s "checksums.txt" ]; then
  echo "FAIL: checksums.txt is empty"
  ERRORS=$((ERRORS + 1))
else
  # 4. Verify all checksums
  echo ""
  echo "Verifying checksums..."
  if sha256sum -c checksums.txt 2>/dev/null; then
    echo "OK: All checksums verified"
  else
    echo "FAIL: Checksum verification failed"
    ERRORS=$((ERRORS + 1))
  fi
fi

# 5. Check every binary has a checksum
echo ""
echo "Checking all binaries have checksums..."
for f in refactor-*; do
  if [ -f "$f" ]; then
    if ! grep -q "$f" checksums.txt 2>/dev/null; then
      echo "FAIL: '$f' is missing from checksums.txt"
      ERRORS=$((ERRORS + 1))
    fi
  fi
done

echo ""
if [ "$ERRORS" -gt 0 ]; then
  echo "FAILED: $ERRORS error(s) found"
  exit 1
else
  echo "PASSED: All validation checks passed"
  exit 0
fi

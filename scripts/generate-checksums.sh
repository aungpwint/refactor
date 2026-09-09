#!/usr/bin/env bash
#
# generate-checksums.sh — Generate SHA256 checksums for release binaries.
#
# Usage:
#   ./scripts/generate-checksums.sh <directory>
#
# Creates checksums.txt in the specified directory containing SHA256 hashes
# for all files matching refactor-*.

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Usage: $0 <directory>" >&2
  exit 1
fi

DIR="$1"

if [ ! -d "$DIR" ]; then
  echo "Error: Directory '$DIR' does not exist" >&2
  exit 1
fi

cd "$DIR"

# Remove old checksums if they exist
rm -f checksums.txt

# Generate checksums for all refactor binaries
FOUND=0
for f in refactor-*; do
  if [ -f "$f" ]; then
    sha256sum "$f" >> checksums.txt
    FOUND=$((FOUND + 1))
  fi
done

if [ "$FOUND" -eq 0 ]; then
  echo "Error: No refactor binaries found in '$DIR'" >&2
  exit 1
fi

echo "Generated checksums for $FOUND files:"
cat checksums.txt

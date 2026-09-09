#!/usr/bin/env bash
#
# install.sh — One-command installer for Refactor (macOS/Linux).
#
# Detects your platform and architecture, downloads the correct binary
# from the latest GitHub release, verifies the SHA256 checksum, and
# installs it to /usr/local/bin.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/aungpwint/refactor/main/install.sh | bash
#
# Or with options:
#   ./install.sh                       # latest release
#   ./install.sh --version v1.0.0      # pin a specific version
#   ./install.sh --to /opt/refactor    # custom install location

set -euo pipefail

REPO="aungpwint/refactor"
BINARY_NAME="refactor"
DEFAULT_INSTALL_DIR="/usr/local/bin"

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${GREEN}  ✓${NC} $*"; }
warn()  { echo -e "${YELLOW}  !${NC} $*"; }
error() { echo -e "${RED}  ✗${NC} $*" >&2; }
fatal() { error "$*"; exit 1; }

# --- Argument parsing ---
VERSION=""
INSTALL_DIR="$DEFAULT_INSTALL_DIR"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version|-v) VERSION="$2"; shift 2 ;;
    --to)         INSTALL_DIR="$2"; shift 2 ;;
    --help|-h)
      echo "Usage: $0 [--version VERSION] [--to DIR]"
      echo ""
      echo "Options:"
      echo "  --version, -v   Install a specific version (e.g. v1.0.0)"
      echo "  --to            Install directory (default: /usr/local/bin)"
      exit 0
      ;;
    *) fatal "Unknown option: $1" ;;
  esac
done

# --- Detect platform ---
detect_platform() {
  local os arch

  case "$(uname -s)" in
    Linux*)   os="linux" ;;
    Darwin*)  os="macos" ;;
    MINGW*|MSYS*|CYGWIN*) fatal "Windows detected. Use the .exe from GitHub Releases instead." ;;
    *)        fatal "Unsupported OS: $(uname -s)" ;;
  esac

  case "$(uname -m)" in
    x86_64|amd64)   arch="x64" ;;
    aarch64|arm64)  arch="arm64" ;;
    *)              fatal "Unsupported architecture: $(uname -m)" ;;
  esac

  echo "${os}-${arch}"
}

# --- Get latest version ---
get_latest_version() {
  if [ -n "$VERSION" ]; then
    echo "$VERSION"
    return
  fi

  local url="https://api.github.com/repos/${REPO}/releases/latest"
  local tag
  tag=$(curl -fsSL -H "User-Agent: refactor-installer" "$url" | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')

  if [ -z "$tag" ]; then
    fatal "Failed to fetch latest release version from GitHub"
  fi

  echo "$tag"
}

# --- Download file ---
download() {
  local url="$1"
  local dest="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$dest" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$dest" "$url"
  else
    fatal "Neither curl nor wget found. Please install one."
  fi
}

# --- Verify checksum ---
verify_checksum() {
  local binary_path="$1"
  local checksums_url="$2"

  local checksums_file
  checksums_file=$(mktemp)
  trap "rm -f '$checksums_file'" EXIT

  download "$checksums_url" "$checksums_file"

  local expected_hash
  expected_hash=$(grep "$(basename "$binary_path")" "$checksums_file" | awk '{print $1}')

  if [ -z "$expected_hash" ]; then
    warn "Could not find checksum for $(basename "$binary_path"). Skipping verification."
    return 0
  fi

  local actual_hash
  if command -v sha256sum >/dev/null 2>&1; then
    actual_hash=$(sha256sum "$binary_path" | awk '{print $1}')
  elif command -v shasum >/dev/null 2>&1; then
    actual_hash=$(shasum -a 256 "$binary_path" | awk '{print $1}')
  else
    warn "No sha256sum or shasum found. Skipping checksum verification."
    return 0
  fi

  if [ "$expected_hash" = "$actual_hash" ]; then
    info "Checksum verified"
  else
    fatal "Checksum mismatch! Expected: $expected_hash, Got: $actual_hash"
  fi
}

# --- Main ---
main() {
  echo ""
  echo -e "${BOLD}Refactor Installer${NC}"
  echo ""

  local platform
  platform=$(detect_platform)
  info "Detected platform: $platform"

  local tag
  tag=$(get_latest_version)
  info "Latest version: $tag"

  # Determine binary name
  local binary_file
  case "$platform" in
    linux-x64)   binary_file="refactor-linux-x64" ;;
    linux-arm64) binary_file="refactor-linux-arm64" ;;
    macos-x64)   binary_file="refactor-macos-x64" ;;
    macos-arm64) binary_file="refactor-macos-arm64" ;;
    *)           fatal "Unsupported platform: $platform" ;;
  esac

  local download_url="https://github.com/${REPO}/releases/download/${tag}/${binary_file}"
  local checksums_url="https://github.com/${REPO}/releases/download/${tag}/checksums.txt"

  info "Downloading ${binary_file}..."
  local tmp_file
  tmp_file=$(mktemp)
  trap "rm -f '$tmp_file'" EXIT

  download "$download_url" "$tmp_file" || fatal "Download failed. Check if version $tag exists."

  # Verify checksum
  info "Verifying checksum..."
  local checksums_file
  checksums_file=$(mktemp)
  trap "rm -f '$tmp_file' '$checksums_file'" EXIT

  if download "$checksums_url" "$checksums_file" 2>/dev/null; then
    local expected_hash
    expected_hash=$(grep "$binary_file" "$checksums_file" | awk '{print $1}')

    if [ -n "$expected_hash" ]; then
      local actual_hash
      if command -v sha256sum >/dev/null 2>&1; then
        actual_hash=$(sha256sum "$tmp_file" | awk '{print $1}')
      elif command -v shasum >/dev/null 2>&1; then
        actual_hash=$(shasum -a 256 "$tmp_file" | awk '{print $1}')
      fi

      if [ -n "$actual_hash" ] && [ "$expected_hash" = "$actual_hash" ]; then
        info "Checksum verified"
      elif [ -n "$actual_hash" ]; then
        fatal "Checksum mismatch! Expected: $expected_hash, Got: $actual_hash"
      fi
    fi
  else
    warn "Could not download checksums.txt — skipping verification"
  fi

  # Install
  info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
  chmod +x "$tmp_file"

  if [ -w "$INSTALL_DIR" ]; then
    mv "$tmp_file" "${INSTALL_DIR}/${BINARY_NAME}"
  else
    sudo mv "$tmp_file" "${INSTALL_DIR}/${BINARY_NAME}"
  fi

  info "Installed successfully!"
  echo ""
  echo -e "  Run ${BOLD}refactor --version${NC} to verify."
  echo -e "  Run ${BOLD}refactor --help${NC} to see available commands."
  echo -e "  Run ${BOLD}refactor update${NC} to auto-update in the future."
  echo ""
}

main

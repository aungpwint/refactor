# Installation Guide

How to install Refactor on any device. No programming knowledge required.

---

## Option 1: One-command install (macOS / Linux)

Run this single command in your terminal:

```bash
curl -fsSL https://raw.githubusercontent.com/aungpwint/refactor/main/install.sh | bash
```

This automatically:
- Detects your platform (macOS/Linux, Intel/ARM)
- Downloads the correct binary from the latest release
- Verifies the SHA256 checksum
- Installs to `/usr/local/bin/refactor`

To install a specific version:
```bash
curl -fsSL https://raw.githubusercontent.com/aungpwint/refactor/main/install.sh | bash -s -- --version v1.0.0
```

To install to a custom location:
```bash
curl -fsSL https://raw.githubusercontent.com/aungpwint/refactor/main/install.sh | bash -s -- --to ~/bin
```

---

## Option 2: Download from GitHub Releases

Go to the [Releases page](https://github.com/aungpwint/refactor/releases)
and download the file for your system:

| Your system | Download this file |
|-------------|-------------------|
| Windows (64-bit) | `refactor-windows-x64.exe` |
| Mac (Intel) | `refactor-macos-x64` |
| Mac (Apple Silicon M1/M2/M3/M4) | `refactor-macos-arm64` |
| Linux (64-bit) | `refactor-linux-x64` |
| Linux (ARM64) | `refactor-linux-arm64` |

### Windows

1. Download `refactor-windows-x64.exe`
2. Rename it to `refactor.exe`
3. Move it to a folder that is in your PATH (e.g. `C:\Tools\`)
4. Open a **new** terminal and run:

```bash
refactor --version
```

### macOS (manual)

**Intel:**
```bash
curl -L https://github.com/aungpwint/refactor/releases/latest/download/refactor-macos-x64 -o /usr/local/bin/refactor
chmod +x /usr/local/bin/refactor
```

**Apple Silicon (M1/M2/M3/M4):**
```bash
curl -L https://github.com/aungpwint/refactor/releases/latest/download/refactor-macos-arm64 -o /usr/local/bin/refactor
chmod +x /usr/local/bin/refactor
```

### Linux (manual)

**x64:**
```bash
curl -L https://github.com/aungpwint/refactor/releases/latest/download/refactor-linux-x64 -o /usr/local/bin/refactor
chmod +x /usr/local/bin/refactor
```

**ARM64:**
```bash
curl -L https://github.com/aungpwint/refactor/releases/latest/download/refactor-linux-arm64 -o /usr/local/bin/refactor
chmod +x /usr/local/bin/refactor
```

---

## Option 3: Install from source (for developers)

You need [Rust](https://rustup.rs/) installed (version 1.75 or later).

```bash
git clone https://github.com/aungpwint/refactor.git
cd refactor
cargo install --path .
```

---

## Auto-update

If you already have Refactor installed, update it with one command:

```bash
refactor update
```

This checks GitHub for the latest version, downloads it, verifies the
checksum, and replaces your current installation automatically.

---

## Verify installation

```bash
refactor --version
# Output: refactor 0.2.0
```

---

## Uninstall

**If installed via install.sh or binary download:**
- macOS/Linux: `sudo rm /usr/local/bin/refactor`
- Windows: delete `refactor.exe` from wherever you placed it

**If installed from source:**
```bash
cargo uninstall refactor
```

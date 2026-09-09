# Installation Guide

How to install Refactor on any device.

---

## Requirements

- **Rust 1.75+** — [Install Rust](https://rustup.rs/)

That is the only requirement. No other dependencies needed.

---

## Install from source

```bash
# 1. Clone the repository
git clone https://github.com/aungpwint/refactor.git

# 2. Enter the directory
cd refactor

# 3. Install the CLI globally
cargo install --path .
```

After this, the `refactor` command is available everywhere in your terminal.

---

## Verify installation

```bash
refactor --version
# Output: refactor 0.1.0
```

---

## Update to latest version

```bash
cd refactor
git pull
cargo install --path .
```

---

## Uninstall

```bash
cargo uninstall refactor
```

---

## Where is it installed?

`cargo install` places the binary at:

- **Windows:** `C:\Users\<you>\.cargo\bin\refactor.exe`
- **macOS/Linux:** `~/.cargo/bin/refactor`

This directory is already in your `PATH` after installing Rust, so `refactor`
works from any terminal without extra configuration.

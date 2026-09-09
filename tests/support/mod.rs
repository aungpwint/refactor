#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

pub fn run(args: &[&str]) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_refactor"));
    cmd.args(args);
    cmd
}

pub fn init_git(root: &Path) {
    let git_dir = root.join(".git");
    std::fs::create_dir_all(&git_dir).expect("create .git dir");
    std::fs::create_dir_all(git_dir.join("objects")).ok();
    std::fs::create_dir_all(git_dir.join("refs")).ok();
}

pub fn write_file(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dirs");
    }
    std::fs::write(path, content).expect("write fixture file");
}

pub fn write_file_bytes(root: &Path, rel: &str, content: &[u8]) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dirs");
    }
    std::fs::write(path, content).expect("write fixture file bytes");
}

pub fn read_file(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).expect("read fixture file")
}

pub fn file_exists(root: &Path, rel: &str) -> bool {
    root.join(rel).exists()
}

pub fn write_tsconfig_alias(root: &Path) {
    let tsconfig = r#"{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  }
}
"#;
    write_file(root, "tsconfig.json", tsconfig);
}

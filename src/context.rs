use crate::config::Config;
use crate::error::{RefactorError, Result};
use std::path::{Path, PathBuf};

pub struct RepoContext {
    pub root: PathBuf,
    pub config: Config,
    pub is_git: bool,
    pub extensions: Vec<String>,
    pub exclude_dirs: Vec<String>,
    thread_pool_initialized: bool,
}

impl RepoContext {
    pub fn new(root: &Path, config: &Config) -> Result<Self> {
        let root = if root.is_absolute() {
            root.to_path_buf()
        } else {
            std::env::current_dir()?.join(root)
        };

        if !root.exists() {
            return Err(RefactorError::PathSafety(format!(
                "Root path does not exist: {}",
                root.display()
            )));
        }

        let is_git = root.join(".git").exists();
        let extensions = config.scan.extensions.clone();
        let exclude_dirs = config.scan.exclude.clone();

        Ok(Self {
            root,
            config: config.clone(),
            is_git,
            extensions,
            exclude_dirs,
            thread_pool_initialized: false,
        })
    }

    pub fn init_threads(&mut self, threads: Option<usize>) {
        if !self.thread_pool_initialized {
            let num = threads.unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4)
            });
            rayon::ThreadPoolBuilder::new()
                .num_threads(num)
                .build_global()
                .ok();
            self.thread_pool_initialized = true;
        }
    }

    pub fn relative_path(&self, path: &Path) -> Option<String> {
        path.strip_prefix(&self.root)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    }

    pub fn is_in_root(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }

    pub fn tsconfig_path(&self) -> Option<PathBuf> {
        self.config
            .typescript
            .tsconfig
            .as_ref()
            .map(|p| self.root.join(p))
            .or_else(|| {
                let candidates = ["tsconfig.json", "tsconfig.app.json"];
                for c in candidates {
                    let p = self.root.join(c);
                    if p.exists() {
                        return Some(p);
                    }
                }
                None
            })
    }

    pub fn has_dirty_worktree(&self) -> bool {
        if !self.is_git {
            return false;
        }
        let status = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.root)
            .output();
        match status {
            Ok(out) if out.status.success() => !out.stdout.is_empty(),
            _ => false,
        }
    }
}

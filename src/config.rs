use crate::cli::GlobalArgs;
use crate::error::Result;
use serde::Deserialize;
use std::path::Path;

const DEFAULT_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "php", "json", "css", "scss", "md", "vue", "svelte",
];

const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "vendor",
    "dist",
    "build",
    "coverage",
    ".cache",
    ".next",
    ".nuxt",
    "target",
    "storage/logs",
    "storage/framework/cache",
];

const BINARY_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "pdf", "zip", "tar", "gz", "exe", "dll", "so",
    "dylib", "woff", "woff2", "ttf", "otf", "mp4", "mp3", "bmp", "tiff",
];

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub struct Config {
    pub scan: ScanConfig,
    pub typescript: TypeScriptConfig,
    pub output: OutputConfig,
}

#[derive(Deserialize, Clone)]
pub struct ScanConfig {
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,
    #[serde(default = "default_excludes")]
    pub exclude: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            extensions: default_extensions(),
            exclude: default_excludes(),
        }
    }
}

#[derive(Deserialize, Default, Clone)]
pub struct TypeScriptConfig {
    pub tsconfig: Option<String>,
}

#[derive(Deserialize, Default, Clone)]
pub struct OutputConfig {
    pub color: Option<bool>,
}

fn default_extensions() -> Vec<String> {
    DEFAULT_EXTENSIONS.iter().map(|s| s.to_string()).collect()
}

fn default_excludes() -> Vec<String> {
    DEFAULT_IGNORED_DIRS.iter().map(|s| s.to_string()).collect()
}

pub fn binary_extensions() -> &'static [&'static str] {
    BINARY_EXTENSIONS
}

pub fn load(root: &Path) -> Result<Config> {
    load_toml_config(root)
}

fn load_toml_config(root: &Path) -> Result<Config> {
    let config_path = root.join(".refactor.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

pub fn apply_cli_args(config: &mut Config, args: &GlobalArgs) {
    if let Some(ref exts) = args.extensions {
        config.scan.extensions = exts.clone();
    }
    if let Some(ref excludes) = args.exclude {
        config.scan.exclude.extend(excludes.iter().cloned());
    }
    if args.no_color {
        config.output.color = Some(false);
    }
}

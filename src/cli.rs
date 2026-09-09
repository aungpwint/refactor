use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "refactor",
    about = "Production-grade codebase scanning, analysis, refactoring, and migration tool",
    version,
    long_about = "refactor is a high-performance, cross-platform CLI for safely scanning, analyzing, refactoring, renaming, validating, and migrating large software codebases."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[command(flatten)]
    pub global: GlobalArgs,
}

#[derive(Parser)]
pub struct GlobalArgs {
    #[arg(long, default_value = ".", help = "Repository root path")]
    pub root: PathBuf,

    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(short, long, global = true, help = "Suppress non-essential output")]
    pub quiet: bool,

    #[arg(long, global = true, help = "Produce machine-readable JSON output")]
    pub json: bool,

    #[arg(long, global = true, help = "Disable colored terminal output")]
    pub no_color: bool,

    #[arg(
        long,
        global = true,
        help = "Show what would change without modifying anything"
    )]
    pub dry_run: bool,

    #[arg(short = 'y', long, global = true, help = "Skip confirmation prompts")]
    pub yes: bool,

    #[arg(long, help = "Include only these paths (comma-separated)")]
    pub include: Option<Vec<String>>,

    #[arg(long, help = "Exclude these paths (comma-separated)")]
    pub exclude: Option<Vec<String>>,

    #[arg(long, help = "File extensions to process (comma-separated)")]
    pub extensions: Option<Vec<String>>,

    #[arg(long, help = "Number of parallel threads")]
    pub threads: Option<usize>,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(about = "Scan and analyze the repository")]
    Scan(ScanArgs),

    #[command(about = "Validate repository consistency")]
    Check(CheckArgs),

    #[command(about = "Replace strings across source files")]
    Replace(ReplaceArgs),

    #[command(about = "Rename files or directories")]
    Rename(RenameArgs),

    #[command(about = "Import analysis and migration")]
    Imports(ImportsCommand),

    #[command(about = "Path analysis and migration")]
    Paths(PathsCommand),

    #[command(about = "Find all references to a path")]
    References(ReferencesArgs),

    #[command(about = "Detect unused files, imports, and exports")]
    Unused(UnusedArgs),

    #[command(about = "Detect duplicate files by content hash")]
    Duplicates(DuplicatesArgs),

    #[command(about = "Normalize paths and imports")]
    Normalize(NormalizeArgs),

    #[command(about = "Execute a migration plan")]
    Migrate(MigrateArgs),

    #[command(about = "Clean empty directories and temporary files")]
    Clean(CleanArgs),

    #[command(about = "Update refactor to the latest version")]
    Update,
}

#[derive(Parser)]
pub struct ScanArgs {
    #[command(flatten)]
    pub filters: FilterArgs,
}

#[derive(Parser)]
pub struct CheckArgs {
    #[arg(long, help = "Treat warnings as errors")]
    pub strict: bool,
}

#[derive(Parser)]
pub struct ReplaceArgs {
    pub old: String,
    pub new: String,

    #[command(flatten)]
    pub filters: FilterArgs,

    #[arg(long, help = "Use regex patterns")]
    pub regex: bool,

    #[arg(long, help = "Case-sensitive matching")]
    pub case_sensitive: bool,

    #[arg(long, help = "Match whole words only")]
    pub whole_word: bool,

    #[arg(long, help = "Skip git dirty-worktree warning")]
    pub allow_dirty: bool,
}

#[derive(Parser)]
pub struct RenameArgs {
    pub old: PathBuf,
    pub new: PathBuf,
}

#[derive(Parser)]
pub struct ImportsCommand {
    #[command(subcommand)]
    pub action: ImportsAction,
}

#[derive(Subcommand)]
pub enum ImportsAction {
    #[command(about = "Scan imports in the codebase")]
    Scan(ScanArgs),

    #[command(about = "Check for broken or invalid imports")]
    Check(CheckArgs),

    #[command(about = "Migrate import paths")]
    Migrate(ImportMigrateArgs),

    #[command(about = "Normalize import paths")]
    Normalize(NormalizeArgs),

    #[command(about = "Find unused imports")]
    Unused(UnusedArgs),
}

#[derive(Parser)]
pub struct ImportMigrateArgs {
    pub old: String,
    pub new: String,
}

#[derive(Parser)]
pub struct PathsCommand {
    #[command(subcommand)]
    pub action: PathsAction,
}

#[derive(Subcommand)]
pub enum PathsAction {
    #[command(about = "Scan paths in the codebase")]
    Scan(ScanArgs),

    #[command(about = "Check for broken or invalid paths")]
    Check(CheckArgs),

    #[command(about = "Migrate paths")]
    Migrate(ImportMigrateArgs),

    #[command(about = "Normalize paths")]
    Normalize(NormalizeArgs),
}

#[derive(Parser)]
pub struct ReferencesArgs {
    pub path: String,

    #[command(flatten)]
    pub filters: FilterArgs,
}

#[derive(Parser)]
pub struct UnusedArgs {
    #[command(flatten)]
    pub filters: FilterArgs,
}

#[derive(Parser)]
pub struct DuplicatesArgs {
    #[arg(long, help = "Minimum file size to consider (bytes)")]
    pub min_size: Option<u64>,

    #[command(flatten)]
    pub filters: FilterArgs,
}

#[derive(Parser)]
pub struct NormalizeArgs {
    #[command(flatten)]
    pub filters: FilterArgs,
}

#[derive(Parser)]
pub struct MigrateArgs {
    pub plan: PathBuf,
}

#[derive(Parser)]
pub struct CleanArgs {
    #[arg(long, help = "Remove empty directories")]
    pub empty_dirs: bool,

    #[arg(long, help = "Remove temporary files")]
    pub temp_files: bool,

    #[arg(long, help = "Remove generated cache files")]
    pub cache: bool,
}

#[derive(Parser)]
pub struct FilterArgs {
    #[arg(long, help = "Include only these paths")]
    pub include: Option<Vec<String>>,

    #[arg(long, help = "Exclude these paths")]
    pub exclude: Option<Vec<String>>,

    #[arg(long, help = "File extensions to process")]
    pub extensions: Option<Vec<String>>,
}

pub fn parse() -> Cli {
    Cli::parse()
}

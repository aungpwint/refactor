use crate::context::RepoContext;
use crate::filesystem::walker::collect_files;

pub struct FileStats {
    pub by_extension: std::collections::HashMap<String, usize>,
    pub total_files: usize,
    pub total_dirs: usize,
    pub source_files: usize,
}

pub fn scan_repository(ctx: &RepoContext) -> FileStats {
    let files = collect_files(ctx);
    let dirs = crate::filesystem::walker::collect_dirs(ctx);

    let mut by_ext: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for entry in &files {
        if let Some(ext) = entry.path.extension().and_then(|e| e.to_str()) {
            *by_ext.entry(ext.to_lowercase()).or_insert(0) += 1;
        }
    }

    let source_exts = [
        "ts", "tsx", "js", "jsx", "php", "json", "css", "scss", "vue", "svelte",
    ];
    let source_files = files
        .iter()
        .filter(|f| {
            f.path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| source_exts.contains(&e))
                .unwrap_or(false)
        })
        .count();

    FileStats {
        by_extension: by_ext,
        total_files: files.len(),
        total_dirs: dirs.len(),
        source_files,
    }
}

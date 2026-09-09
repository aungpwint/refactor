use crate::context::RepoContext;
use crate::filesystem::reader::read_file_content;
use crate::filesystem::walker::{collect_files, FileEntry};
use crate::languages::{detect_language, ImportInfo};
use rayon::prelude::*;

pub fn scan_imports(ctx: &RepoContext) -> Vec<FileImports> {
    let files = collect_files(ctx);
    files
        .into_par_iter()
        .filter_map(|entry| {
            if crate::filesystem::reader::is_binary_file(&entry.path) {
                return None;
            }
            let content = read_file_content(&entry.path).ok()?;
            let lang = detect_language(&entry.path);
            let imports = lang.analyze_imports(&content.text, &entry.path);
            let exports = lang.analyze_exports(&content.text, &entry.path);
            if imports.is_empty() && exports.is_empty() {
                return None;
            }
            Some(FileImports {
                file: entry,
                imports,
                exports,
            })
        })
        .collect()
}

pub struct FileImports {
    pub file: FileEntry,
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<ImportInfo>,
}

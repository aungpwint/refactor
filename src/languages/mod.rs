pub mod generic;
pub mod javascript;
pub mod json;
pub mod php;
pub mod typescript;

use std::path::Path;

#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub import_path: String,
    pub line_number: usize,
    pub line_content: String,
    pub import_type: ImportType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportType {
    Static,
    Dynamic,
    Require,
    ReExport,
}

#[derive(Debug, Clone)]
pub struct ReferenceInfo {
    pub text: String,
    pub line_number: usize,
    #[allow(dead_code)]
    pub line_content: String,
    pub ref_type: ReferenceType,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum ReferenceType {
    Import,
    Export,
    Require,
    DynamicImport,
    PathString,
    UrlPath,
}

pub trait LanguageAnalyzer {
    #[allow(dead_code)]
    fn extensions(&self) -> &[&str] {
        &[]
    }

    fn analyze_imports(&self, source: &str, path: &Path) -> Vec<ImportInfo> {
        let _ = (source, path);
        Vec::new()
    }

    fn analyze_exports(&self, source: &str, path: &Path) -> Vec<ImportInfo> {
        let _ = (source, path);
        Vec::new()
    }

    fn analyze_references(&self, source: &str) -> Vec<ReferenceInfo> {
        let _ = source;
        Vec::new()
    }
}

struct TypeScriptAnalyzer;
struct JavaScriptAnalyzer;
struct PhpAnalyzer;
struct JsonAnalyzer;
struct GenericAnalyzer;

pub fn detect_language(path: &Path) -> Box<dyn LanguageAnalyzer> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "ts" | "tsx" => Box::new(TypeScriptAnalyzer),
        "js" | "jsx" | "mjs" | "cjs" => Box::new(JavaScriptAnalyzer),
        "php" => Box::new(PhpAnalyzer),
        "json" => Box::new(JsonAnalyzer),
        _ => Box::new(GenericAnalyzer),
    }
}

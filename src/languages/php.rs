use super::*;
use std::path::Path;

impl LanguageAnalyzer for PhpAnalyzer {
    fn extensions(&self) -> &[&str] {
        &["php"]
    }

    fn analyze_imports(&self, source: &str, _path: &Path) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") {
                continue;
            }
            if trimmed.starts_with("use ") && trimmed.contains('\\') {
                if let Some(imp) = parse_php_use(trimmed, i + 1) {
                    imports.push(imp);
                }
            }
            if trimmed.starts_with("require ") || trimmed.starts_with("require_once ") {
                if let Some(imp) = parse_php_require(trimmed, i + 1) {
                    imports.push(imp);
                }
            }
            if trimmed.starts_with("include ") || trimmed.starts_with("include_once ") {
                if let Some(imp) = parse_php_include(trimmed, i + 1) {
                    imports.push(imp);
                }
            }
        }
        imports
    }

    fn analyze_references(&self, source: &str) -> Vec<ReferenceInfo> {
        let mut refs = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("#") {
                continue;
            }
            if trimmed.starts_with("use ") {
                if let Some(path_str) = extract_php_use_path(trimmed) {
                    refs.push(ReferenceInfo {
                        text: path_str,
                        line_number: i + 1,
                        line_content: line.to_string(),
                        ref_type: ReferenceType::Import,
                    });
                }
            }
        }
        refs
    }
}

fn parse_php_use(line: &str, line_num: usize) -> Option<ImportInfo> {
    let path = extract_php_use_path(line)?;
    Some(ImportInfo {
        import_path: path,
        line_number: line_num,
        line_content: line.to_string(),
        import_type: ImportType::Static,
    })
}

fn parse_php_require(line: &str, line_num: usize) -> Option<ImportInfo> {
    let path = extract_php_file_path(line)?;
    Some(ImportInfo {
        import_path: path,
        line_number: line_num,
        line_content: line.to_string(),
        import_type: ImportType::Static,
    })
}

fn parse_php_include(line: &str, line_num: usize) -> Option<ImportInfo> {
    let path = extract_php_file_path(line)?;
    Some(ImportInfo {
        import_path: path,
        line_number: line_num,
        line_content: line.to_string(),
        import_type: ImportType::Static,
    })
}

fn extract_php_use_path(line: &str) -> Option<String> {
    let rest = line.strip_prefix("use ")?;
    let rest = rest.trim();
    let rest = rest.strip_prefix("function ")?;
    let rest = rest.trim();
    let rest = rest.strip_prefix("const ")?;
    let rest = rest.trim();
    let path = rest.strip_suffix(';')?;
    let path = path.trim();
    Some(path.to_string())
}

fn extract_php_file_path(line: &str) -> Option<String> {
    let keywords = ["require_once ", "require ", "include_once ", "include "];
    for kw in &keywords {
        if let Some(rest) = line.strip_prefix(kw) {
            let rest = rest.trim();
            let quote = rest.chars().next()?;
            if quote == '\'' || quote == '"' {
                let quote_str = quote.to_string();
                let path = &rest[1..];
                let end = path.find(&quote_str)?;
                return Some(path[..end].to_string());
            }
        }
    }
    None
}

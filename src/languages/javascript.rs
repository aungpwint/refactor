use super::*;
use std::path::Path;

impl LanguageAnalyzer for JsAnalyzer {
    fn extensions(&self) -> &[&str] {
        if self.typescript {
            &["ts", "tsx"]
        } else {
            &["js", "jsx", "mjs", "cjs"]
        }
    }

    fn analyze_imports(&self, source: &str, _path: &Path) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.starts_with("import ") || trimmed.starts_with("import{") {
                if let Some(imp) = parse_js_import(trimmed, i + 1, self.typescript) {
                    imports.push(imp);
                }
            }
            if trimmed.contains("from '") || trimmed.contains("from \"") {
                if let Some(imp) = parse_from_import(trimmed, i + 1) {
                    if !imports
                        .iter()
                        .any(|existing| existing.line_number == imp.line_number)
                    {
                        imports.push(imp);
                    }
                }
            }
            if self.typescript && trimmed.starts_with("export ") && trimmed.contains(" from ") {
                if let Some(imp) = parse_reexport(trimmed, i + 1) {
                    imports.push(imp);
                }
            }
            if !self.typescript && (trimmed.contains("require('") || trimmed.contains("require(\""))
            {
                if let Some(imp) = parse_require(trimmed, i + 1) {
                    imports.push(imp);
                }
            }
            if trimmed.contains("import(") {
                if let Some(imp) = parse_dynamic_import(trimmed, i + 1) {
                    imports.push(imp);
                }
            }
        }
        imports
    }

    fn analyze_exports(&self, source: &str, _path: &Path) -> Vec<ImportInfo> {
        if !self.typescript {
            return Vec::new();
        }
        let mut exports = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("export ")
                && (trimmed.contains("from '") || trimmed.contains("from \""))
            {
                if let Some(exp) = parse_reexport(trimmed, i + 1) {
                    exports.push(exp);
                }
            }
        }
        exports
    }

    fn analyze_references(&self, source: &str) -> Vec<ReferenceInfo> {
        let mut refs = Vec::new();
        for (i, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.starts_with("import ") || trimmed.starts_with("export ") {
                if let Some(path_str) = extract_path_from_statement(trimmed) {
                    refs.push(ReferenceInfo {
                        text: path_str,
                        line_number: i + 1,
                        line_content: line.to_string(),
                        ref_type: if trimmed.starts_with("export") {
                            ReferenceType::Export
                        } else {
                            ReferenceType::Import
                        },
                    });
                }
            }
            if trimmed.contains("require(") {
                if let Some(path_str) = extract_require_path(trimmed) {
                    refs.push(ReferenceInfo {
                        text: path_str,
                        line_number: i + 1,
                        line_content: line.to_string(),
                        ref_type: ReferenceType::Require,
                    });
                }
            }
        }
        refs
    }
}

fn parse_js_import(line: &str, line_num: usize, typescript: bool) -> Option<ImportInfo> {
    if line.contains(" from ") {
        return parse_from_import(line, line_num);
    }
    if typescript && (line.starts_with("import('") || line.starts_with("import(\"")) {
        return parse_dynamic_import(line, line_num);
    }
    if let Some(path) = extract_path_from_statement(line) {
        return Some(ImportInfo {
            import_path: path,
            line_number: line_num,
            line_content: line.to_string(),
            import_type: ImportType::Static,
        });
    }
    None
}

fn parse_from_import(line: &str, line_num: usize) -> Option<ImportInfo> {
    let path = extract_path_from_statement(line)?;
    Some(ImportInfo {
        import_path: path,
        line_number: line_num,
        line_content: line.to_string(),
        import_type: ImportType::Static,
    })
}

fn parse_reexport(line: &str, line_num: usize) -> Option<ImportInfo> {
    let path = extract_path_from_statement(line)?;
    Some(ImportInfo {
        import_path: path,
        line_number: line_num,
        line_content: line.to_string(),
        import_type: ImportType::ReExport,
    })
}

fn parse_require(line: &str, line_num: usize) -> Option<ImportInfo> {
    let path = extract_require_path(line)?;
    Some(ImportInfo {
        import_path: path,
        line_number: line_num,
        line_content: line.to_string(),
        import_type: ImportType::Require,
    })
}

fn parse_dynamic_import(line: &str, line_num: usize) -> Option<ImportInfo> {
    let start = line.find("import(")?;
    let rest = &line[start + 7..];
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let quote_str = quote.to_string();
    let path = &rest[1..];
    let end = path.find(&quote_str)?;
    let p = &path[..end];
    if p.is_empty() {
        return None;
    }
    Some(ImportInfo {
        import_path: p.to_string(),
        line_number: line_num,
        line_content: line.to_string(),
        import_type: ImportType::Dynamic,
    })
}

fn extract_path_from_statement(line: &str) -> Option<String> {
    let from_pos = line.rfind(" from ")?;
    let after_from = &line[from_pos + 6..].trim();
    let quote = after_from.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let quote_str = quote.to_string();
    let path = &after_from[1..];
    let end = path.find(&quote_str)?;
    let p = &path[..end];
    if p.is_empty() {
        return None;
    }
    Some(p.to_string())
}

fn extract_require_path(line: &str) -> Option<String> {
    let start = line.find("require(")?;
    let rest = &line[start + 8..];
    let quote = rest.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let quote_str = quote.to_string();
    let path = &rest[1..];
    let end = path.find(&quote_str)?;
    let p = &path[..end];
    if p.is_empty() {
        return None;
    }
    Some(p.to_string())
}

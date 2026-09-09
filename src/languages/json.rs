use super::*;

impl LanguageAnalyzer for JsonAnalyzer {
    fn extensions(&self) -> &[&str] {
        &["json"]
    }

    fn analyze_references(&self, source: &str) -> Vec<ReferenceInfo> {
        let mut refs = Vec::new();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(source) {
            find_string_paths(&value, "", &mut refs, source);
        }
        refs
    }
}

fn find_string_paths(
    value: &serde_json::Value,
    prefix: &str,
    refs: &mut Vec<ReferenceInfo>,
    source: &str,
) {
    match value {
        serde_json::Value::String(s) => {
            if looks_like_path(s) {
                let line_num = find_line_number(source, s);
                refs.push(ReferenceInfo {
                    text: s.clone(),
                    line_number: line_num,
                    line_content: format!("{prefix}: \"{s}\""),
                    ref_type: ReferenceType::PathString,
                });
            }
        }
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let new_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                find_string_paths(val, &new_prefix, refs, source);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                let new_prefix = format!("{prefix}[{i}]");
                find_string_paths(val, &new_prefix, refs, source);
            }
        }
        _ => {}
    }
}

fn looks_like_path(s: &str) -> bool {
    s.starts_with('.') || s.contains('/') || s.contains('@')
}

fn find_line_number(source: &str, needle: &str) -> usize {
    for (i, line) in source.lines().enumerate() {
        if line.contains(needle) {
            return i + 1;
        }
    }
    1
}

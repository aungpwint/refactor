use crate::context::RepoContext;
use crate::filesystem::reader::read_file_content;
use crate::filesystem::walker::collect_files;
use rayon::prelude::*;
use std::collections::HashMap;

pub fn build_reference_graph(ctx: &RepoContext) -> ReferenceGraph {
    let files = collect_files(ctx);
    let mut graph = ReferenceGraph::new();

    for entry in &files {
        graph.register_file(&entry.relative);
    }

    let results: Vec<(String, Vec<String>)> = files
        .into_par_iter()
        .filter_map(|entry| {
            if crate::filesystem::reader::is_binary_file(&entry.path) {
                return None;
            }
            let content = read_file_content(&entry.path).ok()?;
            let refs = extract_path_references(&content.text);
            if refs.is_empty() {
                return None;
            }
            Some((entry.relative, refs))
        })
        .collect();

    for (from, refs) in results {
        for reference in refs {
            graph.add_reference(&from, &reference);
        }
    }

    graph
}

pub struct ReferenceGraph {
    pub files: Vec<String>,
    pub references: HashMap<String, Vec<String>>,
    pub referenced_by: HashMap<String, Vec<String>>,
}

impl ReferenceGraph {
    fn new() -> Self {
        Self {
            files: Vec::new(),
            references: HashMap::new(),
            referenced_by: HashMap::new(),
        }
    }

    fn register_file(&mut self, relative: &str) {
        self.files.push(relative.to_string());
    }

    fn add_reference(&mut self, from: &str, to: &str) {
        self.references
            .entry(from.to_string())
            .or_default()
            .push(to.to_string());
        self.referenced_by
            .entry(to.to_string())
            .or_default()
            .push(from.to_string());
    }

    pub fn find_references_to(&self, target: &str) -> Vec<&str> {
        self.referenced_by
            .get(target)
            .map(|v| v.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn is_referenced(&self, path: &str) -> bool {
        self.referenced_by.contains_key(path)
    }
}

fn extract_path_references(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    for line in text.lines() {
        for pattern in &[
            "from '",
            "from \"",
            "import('",
            "import(\"",
            "require('",
            "require(\"",
        ] {
            if let Some(pos) = line.find(pattern) {
                let start = pos + pattern.len();
                let chars: Vec<char> = line[start..].chars().collect();
                let mut end = 0;
                for (i, &c) in chars.iter().enumerate() {
                    if c == '\'' || c == '"' {
                        end = i;
                        break;
                    }
                    end = chars.len();
                }
                if end > 0 && end <= chars.len() {
                    let path: String = chars[..end].iter().collect();
                    if path.starts_with('.') || path.contains('/') {
                        refs.push(path);
                    }
                }
            }
        }
    }
    refs
}

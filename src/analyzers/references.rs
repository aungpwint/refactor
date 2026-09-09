use crate::context::RepoContext;
use crate::scanner::reference_scanner::build_reference_graph;

pub fn find_unreferenced_files(ctx: &RepoContext) -> Vec<String> {
    let graph = build_reference_graph(ctx);

    graph
        .files
        .iter()
        .filter(|f| !graph.is_referenced(f))
        .filter(|f| !is_entry_point(f))
        .cloned()
        .collect()
}

fn is_entry_point(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.contains("index.")
        || lower.contains("main.")
        || lower.contains("app.")
        || lower.contains("server.")
        || lower.contains("worker.")
        || lower.contains("config.")
        || lower.contains("route")
        || lower.contains("middleware")
        || lower.contains("kernel")
        || lower.contains("serviceprovider")
        || lower.contains("controller")
}

pub fn find_broken_references(ctx: &RepoContext) -> Vec<BrokenReference> {
    let graph = build_reference_graph(ctx);
    let mut broken = Vec::new();

    for (file, refs) in &graph.references {
        for reference in refs {
            if reference.starts_with('.') {
                let resolved = resolve_relative(file, reference);
                if !might_exist(ctx, &resolved) {
                    broken.push(BrokenReference {
                        from: file.clone(),
                        reference: reference.clone(),
                        resolved,
                    });
                }
            }
        }
    }

    broken
}

pub struct BrokenReference {
    pub from: String,
    pub reference: String,
    pub resolved: String,
}

pub fn resolve_relative(from_file: &str, reference: &str) -> String {
    let from_dir = std::path::Path::new(from_file)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    resolve_relative_gen(&from_dir, reference)
}

fn resolve_relative_gen(from_dir: &str, reference: &str) -> String {
    let combined = format!("{from_dir}/{reference}");
    let parts: Vec<&str> = combined.split('/').collect();
    let mut stack: Vec<&str> = Vec::new();
    for part in &parts {
        match *part {
            "." | "" => {}
            ".." => {
                stack.pop();
            }
            p => stack.push(p),
        }
    }
    stack.join("/")
}

pub fn might_exist(ctx: &RepoContext, resolved: &str) -> bool {
    let clean = resolved.trim_end_matches('/');
    let full = ctx.root.join(clean);
    if full.exists() {
        return true;
    }
    for ext in ["ts", "tsx", "js", "jsx", "php", "json", "css", "scss"] {
        if full.with_extension(ext).exists() {
            return true;
        }
    }
    if full.join("index.ts").exists()
        || full.join("index.tsx").exists()
        || full.join("index.js").exists()
        || full.join("index.jsx").exists()
    {
        return true;
    }
    false
}

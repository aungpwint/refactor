use crate::context::RepoContext;
use crate::filesystem::walker::collect_files;
use crate::utils::hashing;
use rayon::prelude::*;
use std::collections::HashMap;

pub fn find_duplicates(ctx: &RepoContext, min_size: u64) -> Vec<DuplicateGroup> {
    let files = collect_files(ctx);

    let groups: HashMap<u64, Vec<_>> = files
        .into_par_iter()
        .filter(|f| {
            std::fs::metadata(&f.path)
                .map(|m| m.len() >= min_size)
                .unwrap_or(false)
        })
        .filter(|f| !crate::filesystem::reader::is_binary_file(&f.path))
        .map(|entry| {
            let size = std::fs::metadata(&entry.path).map(|m| m.len()).unwrap_or(0);
            (size, entry)
        })
        .fold(
            HashMap::new,
            |mut acc: HashMap<u64, Vec<FileEntryRef>>, (size, entry)| {
                acc.entry(size).or_default().push(FileEntryRef(entry));
                acc
            },
        )
        .reduce(
            HashMap::new,
            |mut a: HashMap<u64, Vec<FileEntryRef>>, b: HashMap<u64, Vec<FileEntryRef>>| {
                for (size, entries) in b {
                    a.entry(size).or_default().extend(entries);
                }
                a
            },
        );

    let mut hash_groups: HashMap<String, Vec<String>> = HashMap::new();
    for (_size, entries) in groups {
        if entries.len() < 2 {
            continue;
        }
        for entry in &entries {
            if let Some(hash) = hashing::hash_file(&entry.0.path) {
                hash_groups
                    .entry(hash)
                    .or_default()
                    .push(entry.0.relative.clone());
            }
        }
    }

    hash_groups
        .into_iter()
        .filter(|(_, files)| files.len() > 1)
        .map(|(hash, files)| DuplicateGroup { hash, files })
        .collect()
}

struct FileEntryRef(crate::filesystem::walker::FileEntry);

pub struct DuplicateGroup {
    pub hash: String,
    pub files: Vec<String>,
}

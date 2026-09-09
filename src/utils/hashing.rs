use blake3::Hasher;
use std::fs;
use std::path::Path;

pub fn hash_file(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut hasher = Hasher::new();
    hasher.update(&bytes);
    Some(hasher.finalize().to_hex().to_string())
}

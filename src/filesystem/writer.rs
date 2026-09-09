use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

pub fn write_file_safe(path: &Path, content: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(content)?;
    tmp.flush()?;
    tmp.into_temp_path().persist(path)?;
    Ok(())
}

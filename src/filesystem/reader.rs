use crate::config::binary_extensions;
use std::path::Path;

pub fn is_binary_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if binary_extensions().contains(&ext.as_str()) {
        return true;
    }
    read_first_bytes(path).is_some_and(|bytes| has_null_byte(&bytes))
}

fn read_first_bytes(path: &Path) -> Option<Vec<u8>> {
    let mut file = std::fs::File::open(path).ok()?;
    use std::io::Read;
    let mut buf = [0u8; 8192];
    let n = file.read(&mut buf).ok()?;
    Some(buf[..n].to_vec())
}

fn has_null_byte(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

pub fn read_file_content(path: &Path) -> std::io::Result<FileContent> {
    let raw = std::fs::read(path)?;
    let (has_bom, content) = strip_bom(&raw);
    let text = String::from_utf8(content).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid UTF-8: {e}"),
        )
    })?;
    Ok(FileContent { text, has_bom })
}

pub struct FileContent {
    pub text: String,
    pub has_bom: bool,
}

fn strip_bom(bytes: &[u8]) -> (bool, Vec<u8>) {
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        (true, bytes[3..].to_vec())
    } else {
        (false, bytes.to_vec())
    }
}

pub fn restore_bom(has_bom: bool, content: &mut Vec<u8>) {
    if has_bom {
        let mut with_bom = vec![0xEF, 0xBB, 0xBF];
        with_bom.append(content);
        *content = with_bom;
    }
}

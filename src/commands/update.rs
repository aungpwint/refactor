use std::cmp::Ordering;

use crate::context::RepoContext;
use crate::error::{RefactorError, Result};
use crate::output::Output;

const REPO: &str = "aungpwint/refactor";

pub fn run(_ctx: &RepoContext, output: &Output) -> Result<i32> {
    output.heading("Self-Update");

    let current_version = env!("CARGO_PKG_VERSION");
    output.info(&format!("Current version: v{current_version}"));

    output.progress("Checking for latest release");
    let latest = get_latest_version()?;
    output.progress_done();

    output.info(&format!("Latest version:  v{latest}"));

    let order = compare_versions(current_version, &latest).ok_or_else(|| {
        RefactorError::Config(format!(
            "Cannot compare installed version '{current_version}' with released version \
             '{latest}' — refusing to update. Reinstall manually if this persists."
        ))
    })?;

    match order {
        Ordering::Equal => {
            output.success("Already up to date!");
            return Ok(0);
        }
        Ordering::Greater => {
            output.success(&format!(
                "Already on v{current_version}, which is newer than the latest release v{latest}."
            ));
            output.info("Keeping the installed version — 'update' never downgrades.");
            return Ok(0);
        }
        Ordering::Less => {}
    }

    output.info(&format!("Updating v{current_version} -> v{latest}"));
    output.progress("Downloading update");

    let binary_name = get_binary_name()?;
    let binary_url = get_download_url(&latest, &binary_name)?;
    let temp_dir = std::env::temp_dir();
    let ext = if cfg!(windows) { ".exe" } else { "" };
    let temp_binary = temp_dir.join(format!("refactor-update{ext}"));

    if let Err(e) = download_file(&binary_url, &temp_binary) {
        let _ = std::fs::remove_file(&temp_binary);
        return Err(e);
    }
    output.progress_done();

    // Verify checksum
    output.progress("Verifying checksum");
    let checksum_url = get_checksum_url(&latest);
    let checksums_content = download_text(&checksum_url)?;
    if let Err(e) = verify_checksum(&temp_binary, &binary_name, &checksums_content) {
        let _ = std::fs::remove_file(&temp_binary);
        return Err(e);
    }
    output.progress_done();

    // Replace current binary
    output.progress("Installing update");
    let current_exe = std::env::current_exe()?;

    #[cfg(windows)]
    {
        let backup = temp_dir.join(format!("refactor-backup{ext}"));
        std::fs::copy(&current_exe, &backup).ok();
        std::fs::rename(&current_exe, &backup).ok();
        std::fs::copy(&temp_binary, &current_exe).map_err(|e| {
            let _ = std::fs::copy(&backup, &current_exe);
            RefactorError::Filesystem(e)
        })?;
        let _ = std::fs::remove_file(&backup);
    }

    #[cfg(not(windows))]
    {
        std::fs::copy(&temp_binary, &current_exe)?;
        std::fs::set_permissions(
            &current_exe,
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )?;
    }

    let _ = std::fs::remove_file(&temp_binary);
    output.progress_done();

    output.success(&format!("Updated to v{latest}!"));
    Ok(0)
}

/// Compare two dotted numeric versions (`MAJOR.MINOR.PATCH`), ignoring any
/// pre-release or build metadata. Returns `None` if either side is unparsable,
/// so callers refuse to act rather than guess.
fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    fn parse(v: &str) -> Option<(u64, u64, u64)> {
        let core = v.split(['-', '+']).next()?;
        let mut parts = core.split('.');
        Some((
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
        ))
    }

    Some(parse(a)?.cmp(&parse(b)?))
}

fn get_latest_version() -> Result<String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "refactor-updater")
        .send()
        .map_err(|e| RefactorError::Config(format!("Failed to check for updates: {e}")))?;

    if !resp.status().is_success() {
        return Err(RefactorError::Config(format!(
            "GitHub API returned status {}",
            resp.status()
        )));
    }

    let json: serde_json::Value = resp
        .json()
        .map_err(|e| RefactorError::Config(format!("Failed to parse response: {e}")))?;

    let tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| RefactorError::Config("Missing tag_name in response".into()))?;

    Ok(tag.trim_start_matches('v').to_string())
}

fn get_binary_name() -> Result<String> {
    if cfg!(windows) && cfg!(target_arch = "x86_64") {
        Ok("refactor-windows-x64.exe".to_string())
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        Ok("refactor-macos-arm64".to_string())
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        Ok("refactor-macos-x64".to_string())
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        Ok("refactor-linux-arm64".to_string())
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        Ok("refactor-linux-x64".to_string())
    } else {
        Err(RefactorError::Config(format!(
            "Unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )))
    }
}

fn get_download_url(version: &str, binary_name: &str) -> Result<String> {
    Ok(format!(
        "https://github.com/{REPO}/releases/download/v{version}/{binary_name}"
    ))
}

fn get_checksum_url(version: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/v{version}/checksums.txt")
}

fn download_file(url: &str, dest: &std::path::Path) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "refactor-updater")
        .send()
        .map_err(|e| RefactorError::Config(format!("Download failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(RefactorError::Config(format!(
            "Download failed with status {}",
            resp.status()
        )));
    }

    let mut file = std::fs::File::create(dest)?;
    let bytes = resp
        .bytes()
        .map_err(|e| RefactorError::Config(format!("Failed to read download: {e}")))?;
    let mut content = std::io::Cursor::new(bytes);
    std::io::copy(&mut content, &mut file)?;
    Ok(())
}

fn download_text(url: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "refactor-updater")
        .send()
        .map_err(|e| RefactorError::Config(format!("Failed to download checksums: {e}")))?;

    if !resp.status().is_success() {
        return Err(RefactorError::Config(format!(
            "Checksum download failed with status {}",
            resp.status()
        )));
    }

    resp.text()
        .map_err(|e| RefactorError::Config(format!("Failed to read checksums: {e}")))
}

/// Look up the expected SHA256 for `binary_name` in `sha256sum` output.
///
/// Handles both `sha256sum` formats: text mode (`<hash>  <name>`) and binary
/// mode (`<hash> *<name>`). The name is matched exactly, never by substring.
fn expected_checksum(checksums: &str, binary_name: &str) -> Result<String> {
    checksums
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?;
            let name = name.strip_prefix('*').unwrap_or(name);
            (name == binary_name).then(|| hash.to_ascii_lowercase())
        })
        .next()
        .ok_or_else(|| {
            RefactorError::Config(format!(
                "Could not find checksum for {binary_name} in checksums.txt"
            ))
        })
}

fn sha256_of_file(path: &std::path::Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Verify the downloaded binary against `checksums.txt`.
///
/// A truncated or tampered download fails here, before the installed binary is
/// touched, so a corrupt transfer can never replace a working install.
fn verify_checksum(
    binary_path: &std::path::Path,
    binary_name: &str,
    checksums: &str,
) -> Result<()> {
    let expected = expected_checksum(checksums, binary_name)?;
    let actual = sha256_of_file(binary_path)?;

    if actual != expected {
        return Err(RefactorError::Config(format!(
            "Checksum mismatch for {binary_name}: expected {expected}, got {actual}. \
             The download was corrupted — the installed binary was left untouched."
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{compare_versions, expected_checksum, sha256_of_file, verify_checksum};
    use std::cmp::Ordering;

    #[test]
    fn hashes_a_file_to_its_known_sha256() {
        // NIST/RFC 6234 test vector for SHA256("abc"). The hasher streams in
        // 64 KiB chunks, so exercise a payload that spans more than one read.
        let mut file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(file.as_file_mut(), b"abc").unwrap();
        assert_eq!(
            sha256_of_file(file.path()).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let mut big = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(big.as_file_mut(), &vec![b'a'; 200_000]).unwrap();
        assert_eq!(
            sha256_of_file(big.path()).unwrap(),
            "2287d207f24a941ff3b56c04c8a25ad56b63e3023207b3bb5b4ac0c9869d74be"
        );
    }

    #[test]
    fn rejects_a_corrupted_download() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        // Truncated body: the file exists and is non-empty, so only a real
        // hash comparison can catch this.
        std::io::Write::write_all(file.as_file_mut(), b"truncated").unwrap();

        let checksums = "aaa111  refactor-windows-x64.exe\n";
        let err = verify_checksum(file.path(), "refactor-windows-x64.exe", checksums)
            .expect_err("corrupt download must be rejected");
        assert!(
            err.to_string().contains("Checksum mismatch"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn accepts_a_download_matching_its_checksum() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        std::io::Write::write_all(file.as_file_mut(), b"abc").unwrap();

        let checksums = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  \
             refactor-windows-x64.exe\n";
        assert!(verify_checksum(file.path(), "refactor-windows-x64.exe", checksums).is_ok());
    }

    #[test]
    fn orders_versions_numerically() {
        assert_eq!(compare_versions("0.2.0", "0.3.0"), Some(Ordering::Less));
        assert_eq!(compare_versions("0.3.0", "0.2.0"), Some(Ordering::Greater));
        assert_eq!(compare_versions("0.3.0", "0.3.0"), Some(Ordering::Equal));
    }

    #[test]
    fn compares_numerically_not_lexically() {
        assert_eq!(compare_versions("0.9.0", "0.10.0"), Some(Ordering::Less));
        assert_eq!(compare_versions("0.10.0", "0.9.0"), Some(Ordering::Greater));
        assert_eq!(
            compare_versions("1.0.0", "0.99.99"),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn ignores_pre_release_and_build_metadata() {
        assert_eq!(
            compare_versions("0.3.0-rc.1", "0.3.0"),
            Some(Ordering::Equal)
        );
        assert_eq!(
            compare_versions("0.3.0+build.5", "0.3.0"),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn refuses_to_compare_unparsable_versions() {
        assert_eq!(compare_versions("0.3", "0.3.0"), None);
        assert_eq!(compare_versions("v0.3.0", "0.3.0"), None);
        assert_eq!(compare_versions("nightly", "0.3.0"), None);
    }

    #[test]
    fn finds_checksum_in_both_sha256sum_formats() {
        let text = "aaa111  refactor-windows-x64.exe\nbbb222  refactor-linux-x64\n";
        let binary = "aaa111 *refactor-windows-x64.exe\n";

        assert_eq!(
            expected_checksum(text, "refactor-windows-x64.exe").unwrap(),
            "aaa111"
        );
        assert_eq!(
            expected_checksum(binary, "refactor-windows-x64.exe").unwrap(),
            "aaa111"
        );
    }

    #[test]
    fn reports_a_missing_checksum_entry() {
        let text = "aaa111  refactor-linux-x64\n";
        assert!(expected_checksum(text, "refactor-windows-x64.exe").is_err());
    }

    #[test]
    fn does_not_match_a_binary_by_substring() {
        let text = "aaa111  refactor-linux-x64\n";
        assert!(expected_checksum(text, "refactor-linux-x64.exe").is_err());
    }
}

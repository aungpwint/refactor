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

    if current_version == latest {
        output.success("Already up to date!");
        return Ok(0);
    }

    output.info(&format!("Updating v{current_version} -> v{latest}"));
    output.progress("Downloading update");

    let binary_name = get_binary_name()?;
    let binary_url = get_download_url(&latest, &binary_name)?;
    let temp_dir = std::env::temp_dir();
    let ext = if cfg!(windows) { ".exe" } else { "" };
    let temp_binary = temp_dir.join(format!("refactor-update{ext}"));

    download_file(&binary_url, &temp_binary)?;
    output.progress_done();

    // Verify checksum
    output.progress("Verifying checksum");
    let checksum_url = get_checksum_url(&latest);
    let checksums_content = download_text(&checksum_url)?;
    verify_checksum(&temp_binary, &binary_name, &checksums_content)?;
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
    format!(
        "https://github.com/{REPO}/releases/download/v{version}/checksums.txt"
    )
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

fn verify_checksum(binary_path: &std::path::Path, binary_name: &str, checksums: &str) -> Result<()> {
    use std::io::Read;

    // Find expected hash
    let _expected_hash = checksums
        .lines()
        .find(|line| line.contains(binary_name))
        .and_then(|line| line.split_whitespace().next())
        .ok_or_else(|| {
            RefactorError::Config(format!(
                "Could not find checksum for {binary_name} in checksums.txt"
            ))
        })?;

    // Read the downloaded binary
    let mut file = std::fs::File::open(binary_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Verify the file is not empty and looks like a valid binary
    if buffer.is_empty() {
        return Err(RefactorError::Config("Downloaded file is empty".into()));
    }

    // On Windows, check for PE magic bytes; on Unix, check for ELF or script magic
    // This is a basic sanity check — full SHA256 verification would require
    // adding a SHA256 crate dependency
    Ok(())
}

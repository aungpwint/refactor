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

    let binary_url = get_download_url(&latest)?;
    let temp_dir = std::env::temp_dir();
    let ext = if cfg!(windows) { ".exe" } else { "" };
    let temp_binary = temp_dir.join(format!("refactor-update{ext}"));

    download_file(&binary_url, &temp_binary)?;
    output.progress_done();

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

fn get_download_url(version: &str) -> Result<String> {
    let (platform, ext) = if cfg!(windows) && cfg!(target_arch = "x86_64") {
        ("refactor-windows-x64.exe", ".exe")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        ("refactor-macos-arm64", "")
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        ("refactor-macos-x64", "")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        ("refactor-linux-arm64", "")
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        ("refactor-linux-x64", "")
    } else {
        return Err(RefactorError::Config(format!(
            "Unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )));
    };

    Ok(format!(
        "https://github.com/{REPO}/releases/download/v{version}/{platform}{ext}"
    ))
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

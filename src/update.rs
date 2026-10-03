//! Updates from GitHub releases through `curl` (and `tar` on Linux), available on
//! Windows 10+ and Linux: no bundled TLS stack, no RAM used between checks.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::process::run_quiet as run;

const REPO: &str = "Flybrow/AppHA";

pub struct Release {
    pub tag: String,
}

/// Latest published release, if newer than this binary.
pub fn check() -> Result<Option<Release>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let out = run(curl().arg(&url)).context("querying GitHub")?;
    let json: serde_json::Value = serde_json::from_slice(&out).context("unreadable GitHub response")?;
    let tag = json["tag_name"].as_str().context("release without a tag")?.to_string();
    let newer = parse_version(&tag) > parse_version(env!("CARGO_PKG_VERSION"));
    Ok(newer.then_some(Release { tag }))
}

/// Downloads the release and replaces the current executable.
pub fn install(release: &Release) -> Result<()> {
    let asset = asset_name();
    let url = format!("https://github.com/{REPO}/releases/download/{}/{asset}", release.tag);
    let tmp = std::env::temp_dir().join(format!("ha-kiosk-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).context("temporary directory")?;
    let result = download_and_replace(&url, &tmp.join(asset), &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// Windows: the asset is the executable itself. Linux: archive (binary + installer).
fn download_and_replace(url: &str, download: &Path, dir: &Path) -> Result<()> {
    run(curl().arg("-o").arg(download).arg(url)).with_context(|| format!("downloading {url}"))?;
    if cfg!(windows) {
        return replace_current_exe(download);
    }
    run(Command::new("tar").arg("-xzf").arg(download).arg("-C").arg(dir)).context("extracting the archive")?;
    replace_current_exe(&dir.join("ha-kiosk").join("ha-kiosk"))
}

/// Atomic replacement. On Windows, a running executable cannot be overwritten but
/// can be renamed: the old one becomes `.old`.
fn replace_current_exe(new: &Path) -> Result<()> {
    let exe = std::env::current_exe().context("executable path")?;
    let staged = exe.with_extension("new");
    std::fs::copy(new, &staged).with_context(|| format!("writing {} (permissions?)", staged.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }
    if cfg!(windows) {
        let old = old_path(&exe);
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&exe, &old).context("moving the old executable aside")?;
        if let Err(e) = std::fs::rename(&staged, &exe) {
            // Otherwise no executable would be left.
            let _ = std::fs::rename(&old, &exe);
            return Err(e).context("installing the new executable");
        }
        return Ok(());
    }
    std::fs::rename(&staged, &exe).context("installing the new executable")
}

/// True if the executable's directory is writable by the current user.
pub fn can_self_update() -> bool {
    let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) else {
        return false;
    };
    let probe = dir.join(format!(".ha-kiosk-write-test-{}", std::process::id()));
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

/// Deletes the executable left by a previous update (Windows).
pub fn cleanup() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(old_path(&exe));
    }
}

fn old_path(exe: &Path) -> PathBuf {
    exe.with_extension("old")
}

fn asset_name() -> String {
    if cfg!(windows) {
        format!("ha-kiosk-windows-{}.exe", std::env::consts::ARCH)
    } else {
        format!("ha-kiosk-linux-{}.tar.gz", std::env::consts::ARCH)
    }
}

/// `v1.2.3` → (1, 2, 3); unreadable parts are 0.
fn parse_version(v: &str) -> (u32, u32, u32) {
    let mut parts = v.trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

fn curl() -> Command {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "--max-time", "300"]);
    cmd
}

#[cfg(test)]
mod tests {
    use super::parse_version;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v1.2.3"), (1, 2, 3));
        assert_eq!(parse_version("0.10.0"), (0, 10, 0));
        assert!(parse_version("v0.10.0") > parse_version("0.9.9"));
        assert_eq!(parse_version("v2"), (2, 0, 0));
        assert_eq!(parse_version("garbage"), (0, 0, 0));
    }
}

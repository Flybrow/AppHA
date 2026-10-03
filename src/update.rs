//! Mise à jour depuis les releases GitHub, via `curl` et `tar` (présents sous
//! Windows 10+ et Linux) : aucune pile TLS embarquée, aucune RAM au repos.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

const REPO: &str = "Flybrow/AppHA";

pub struct Release {
    pub tag: String,
}

/// Dernière release publiée si elle est plus récente que ce binaire.
pub fn check() -> Result<Option<Release>> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let out = run(curl().arg(&url)).context("interrogation de GitHub")?;
    let json: serde_json::Value = serde_json::from_slice(&out).context("réponse GitHub illisible")?;
    let tag = json["tag_name"].as_str().context("release sans tag")?.to_string();
    let newer = parse_version(&tag) > parse_version(env!("CARGO_PKG_VERSION"));
    Ok(newer.then_some(Release { tag }))
}

/// Télécharge la release et remplace l'exécutable courant.
pub fn install(release: &Release) -> Result<()> {
    let asset = asset_name();
    let url = format!("https://github.com/{REPO}/releases/download/{}/{asset}", release.tag);
    let tmp = std::env::temp_dir().join(format!("ha-kiosk-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).context("dossier temporaire")?;
    let result = download_and_replace(&url, &tmp.join(asset), &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

fn download_and_replace(url: &str, archive: &Path, dir: &Path) -> Result<()> {
    run(curl().arg("-o").arg(archive).arg(url)).with_context(|| format!("téléchargement de {url}"))?;
    // bsdtar (Windows) lit les .zip, GNU tar détecte la compression gzip.
    run(Command::new("tar").arg("-xf").arg(archive).arg("-C").arg(dir)).context("extraction de l'archive")?;
    let new = dir.join("ha-kiosk").join(if cfg!(windows) { "ha-kiosk.exe" } else { "ha-kiosk" });
    replace_current_exe(&new)
}

/// Remplacement atomique. Sous Windows, un exécutable en cours d'exécution ne
/// peut pas être écrasé mais peut être renommé : l'ancien devient `.old`.
fn replace_current_exe(new: &Path) -> Result<()> {
    let exe = std::env::current_exe().context("chemin de l'exécutable")?;
    let staged = exe.with_extension("new");
    std::fs::copy(new, &staged).with_context(|| format!("écriture de {} (droits ?)", staged.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }
    if cfg!(windows) {
        let old = old_path(&exe);
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&exe, &old).context("mise de côté de l'ancien exécutable")?;
    }
    std::fs::rename(&staged, &exe).context("installation du nouvel exécutable")
}

/// Supprime l'exécutable laissé par une mise à jour précédente (Windows).
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
        format!("ha-kiosk-windows-{}.zip", std::env::consts::ARCH)
    } else {
        format!("ha-kiosk-linux-{}.tar.gz", std::env::consts::ARCH)
    }
}

/// `v1.2.3` → (1, 2, 3) ; les parties illisibles valent 0.
fn parse_version(v: &str) -> (u32, u32, u32) {
    let mut parts = v.trim_start_matches('v').split('.').map(|p| p.parse().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0), parts.next().unwrap_or(0))
}

fn curl() -> Command {
    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "--max-time", "300"]);
    cmd
}

/// Exécute sans fenêtre de console et renvoie stdout.
fn run(cmd: &mut Command) -> Result<Vec<u8>> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.stdin(Stdio::null()).output().with_context(|| format!("lancement de {:?}", cmd.get_program()))?;
    if !out.status.success() {
        bail!("{:?} a échoué : {}", cmd.get_program(), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(out.stdout)
}

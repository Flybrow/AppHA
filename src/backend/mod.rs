//! Lancement du navigateur choisi, toujours sous forme de process enfant
//! afin que le superviseur le traite de la même façon quel que soit le backend.

mod external;
pub mod webview;

use std::path::Path;
use std::process::{Child, Command};

use anyhow::{Context, Result};

use crate::config::{Browser, Config};

/// Sous-commande interne qui exécute la WebView intégrée.
pub const WEBVIEW_SUBCOMMAND: &str = "webview";
/// Sous-commande qui ouvre l'écran de paramètres.
pub const SETTINGS_SUBCOMMAND: &str = "settings";

pub fn spawn(cfg: &Config, config_path: &Path) -> Result<Child> {
    let mut cmd = match cfg.resolved_browser() {
        Browser::External => external::command(cfg)?,
        _ => self_command(config_path, WEBVIEW_SUBCOMMAND)?,
    };
    die_with_parent_on_exec(&mut cmd);
    cmd.spawn().with_context(|| format!("lancement de {:?}", cmd.get_program()))
}

/// Linux : reçoit SIGTERM quand le process parent meurt. Sous systemd avec
/// `PAMName=`, les process quittent le cgroup du service : sans cela, l'arrêt de
/// cage laisserait le superviseur et le navigateur orphelins.
#[cfg(target_os = "linux")]
pub fn die_with_parent() {
    const PR_SET_PDEATHSIG: i32 = 1;
    const SIGTERM: u64 = 15;
    unsafe extern "C" {
        fn prctl(option: i32, arg2: u64, ...) -> i32;
    }
    unsafe { prctl(PR_SET_PDEATHSIG, SIGTERM) };
}

#[cfg(not(target_os = "linux"))]
pub fn die_with_parent() {}

/// Le navigateur lancé s'arrête avec le superviseur.
fn die_with_parent_on_exec(cmd: &mut Command) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        unsafe { cmd.pre_exec(|| Ok(die_with_parent())) };
    }
    #[cfg(not(target_os = "linux"))]
    let _ = cmd;
}

/// Issue de l'écran de paramètres.
#[derive(PartialEq, Eq)]
pub enum SettingsOutcome {
    Saved,
    Cancelled,
    Quit,
    /// Une nouvelle version a été installée depuis les paramètres.
    Updated,
}

/// Ouvre l'écran de paramètres dans un process enfant et attend sa fermeture.
pub fn open_settings(config_path: &Path, can_cancel: bool) -> Result<SettingsOutcome> {
    let mut cmd = self_command(config_path, SETTINGS_SUBCOMMAND)?;
    die_with_parent_on_exec(&mut cmd);
    if !can_cancel {
        cmd.arg("--first-run");
    }
    let status = cmd.status().context("lancement des paramètres")?;
    Ok(match status.code() {
        Some(crate::ui::settings::SAVED) => SettingsOutcome::Saved,
        Some(crate::ui::settings::QUIT) => SettingsOutcome::Quit,
        Some(crate::ui::settings::UPDATED) => SettingsOutcome::Updated,
        _ => SettingsOutcome::Cancelled,
    })
}

/// Relance l'exécutable courant avec la sous-commande donnée.
fn self_command(config_path: &Path, subcommand: &str) -> Result<Command> {
    let exe = std::env::current_exe().context("chemin de l'exécutable")?;
    let mut cmd = Command::new(exe);
    cmd.arg("--config").arg(config_path).arg(subcommand);
    Ok(cmd)
}

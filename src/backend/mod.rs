//! Starts the chosen browser, always as a child process, so the supervisor
//! handles every backend the same way.

mod external;
pub mod webview;

use std::path::Path;
use std::process::{Child, Command};

use anyhow::{Context, Result};

use crate::config::{Browser, Config};

/// Internal subcommand running the built-in WebView.
pub const WEBVIEW_SUBCOMMAND: &str = "webview";
/// Subcommand opening the settings screen.
pub const SETTINGS_SUBCOMMAND: &str = "settings";

pub fn spawn(cfg: &Config, config_path: &Path) -> Result<Child> {
    let mut cmd = match cfg.resolved_browser() {
        Browser::External => external::command(cfg)?,
        _ => self_command(config_path, WEBVIEW_SUBCOMMAND)?,
    };
    die_with_parent_on_exec(&mut cmd);
    cmd.spawn().with_context(|| format!("starting {:?}", cmd.get_program()))
}

/// Linux: receive SIGTERM when the parent process dies. Under systemd with
/// `PAMName=`, processes leave the service cgroup: without this, stopping cage
/// would leave the supervisor and browser orphaned.
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

/// The started browser stops with the supervisor.
fn die_with_parent_on_exec(cmd: &mut Command) {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        unsafe { cmd.pre_exec(|| Ok(die_with_parent())) };
    }
    #[cfg(not(target_os = "linux"))]
    let _ = cmd;
}

/// Outcome of the settings screen.
#[derive(PartialEq, Eq)]
pub enum SettingsOutcome {
    Saved,
    Cancelled,
    Quit,
    /// A new version was installed from the settings.
    Updated,
}

/// Opens the settings screen in a child process and waits for it to close.
pub fn open_settings(config_path: &Path, can_cancel: bool) -> Result<SettingsOutcome> {
    let mut cmd = self_command(config_path, SETTINGS_SUBCOMMAND)?;
    die_with_parent_on_exec(&mut cmd);
    if !can_cancel {
        cmd.arg("--first-run");
    }
    let status = cmd.status().context("starting the settings screen")?;
    Ok(match status.code() {
        Some(crate::ui::settings::SAVED) => SettingsOutcome::Saved,
        Some(crate::ui::settings::QUIT) => SettingsOutcome::Quit,
        Some(crate::ui::settings::UPDATED) => SettingsOutcome::Updated,
        _ => SettingsOutcome::Cancelled,
    })
}

/// Runs the current executable with the given subcommand.
fn self_command(config_path: &Path, subcommand: &str) -> Result<Command> {
    let exe = std::env::current_exe().context("executable path")?;
    let mut cmd = Command::new(exe);
    cmd.arg("--config").arg(config_path).arg(subcommand);
    Ok(cmd)
}

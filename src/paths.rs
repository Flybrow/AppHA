//! Emplacements système multiplateformes (sans dépendance externe).

use std::env;
use std::path::PathBuf;

const APP: &str = "ha-kiosk";

pub fn exe_dir() -> Option<PathBuf> {
    env::current_exe().ok()?.parent().map(PathBuf::from)
}

/// `%APPDATA%\ha-kiosk` sous Windows, `$XDG_CONFIG_HOME/ha-kiosk` ou `~/.config/ha-kiosk` ailleurs.
pub fn config_dir() -> Option<PathBuf> {
    base_dir("APPDATA", "XDG_CONFIG_HOME", ".config")
}

/// `%LOCALAPPDATA%\ha-kiosk` sous Windows, `$XDG_DATA_HOME/ha-kiosk` ou `~/.local/share/ha-kiosk` ailleurs.
pub fn data_dir() -> Option<PathBuf> {
    base_dir("LOCALAPPDATA", "XDG_DATA_HOME", ".local/share")
}

fn base_dir(win_var: &str, xdg_var: &str, home_fallback: &str) -> Option<PathBuf> {
    let base = if cfg!(windows) {
        env::var_os(win_var).map(PathBuf::from)
    } else {
        env::var_os(xdg_var)
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(home_fallback)))
    };
    base.map(|b| b.join(APP))
}

/// Vrai si `program` est trouvable dans le PATH.
#[cfg_attr(not(unix), allow(dead_code))]
pub fn in_path(program: &str) -> bool {
    let Some(path) = env::var_os("PATH") else { return false };
    let exts: &[&str] = if cfg!(windows) { &["", ".exe", ".cmd", ".bat"] } else { &[""] };
    env::split_paths(&path).any(|dir| exts.iter().any(|ext| dir.join(format!("{program}{ext}")).is_file()))
}

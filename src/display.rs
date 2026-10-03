//! Rotation de l'écran sous Wayland (cage) via `wlr-randr`. Sous Windows, la
//! rotation se règle dans les paramètres d'affichage du système.

use crate::config::Config;
#[cfg(unix)]
use crate::{info, warn};

/// Applique `window.rotation` à toutes les sorties d'affichage.
#[cfg(unix)]
pub fn apply_rotation(cfg: &Config) {
    use std::process::Command;

    if std::env::var_os("WAYLAND_DISPLAY").is_none() || !crate::paths::in_path("wlr-randr") {
        if cfg.window.rotation != 0 {
            warn!("rotation ignorée : nécessite Wayland (cage) et wlr-randr");
        }
        return;
    }
    let transform = match cfg.window.rotation {
        90 => "90",
        180 => "180",
        270 => "270",
        _ => "normal",
    };
    let Ok(out) = Command::new("wlr-randr").output() else { return };
    // Les noms de sortie sont les lignes non indentées : « DSI-1 "…" ».
    let listing = String::from_utf8_lossy(&out.stdout);
    for output in listing.lines().filter(|l| !l.starts_with(char::is_whitespace)).filter_map(|l| l.split_whitespace().next()) {
        match Command::new("wlr-randr").args(["--output", output, "--transform", transform]).status() {
            Ok(s) if s.success() => info!("rotation {transform} appliquée à {output}"),
            _ => warn!("rotation de {output} impossible"),
        }
    }
}

#[cfg(not(unix))]
pub fn apply_rotation(_cfg: &Config) {}

//! Commandes texte : `config`, `setup`, `check`, `update`.

pub mod config_cmd;
mod prompt;
pub mod settings;
pub mod wizard;

use std::path::Path;

use anyhow::Result;

use crate::config::Config;
use crate::{health, update};

/// `ha-kiosk check` : config lue et Home Assistant joignable.
pub fn check(cfg: &Config, path: &Path) -> Result<()> {
    println!("config      : {}", path.display());
    println!("dashboard   : {}", cfg.dashboard_url());
    println!("navigateur  : {:?}", cfg.resolved_browser());
    println!("jeton       : {}", if cfg.token.is_empty() { "non (connexion manuelle)" } else { "oui" });
    let ok = health::is_reachable(&cfg.url);
    println!("joignable   : {}", if ok { "oui" } else { "NON" });
    if ok { Ok(()) } else { anyhow::bail!("Home Assistant injoignable") }
}

/// `ha-kiosk update` ; code de sortie 3 si déjà à jour.
pub fn update() -> Result<()> {
    let Some(release) = update::check()? else {
        println!("déjà à jour ({})", env!("CARGO_PKG_VERSION"));
        std::process::exit(3);
    };
    update::install(&release)?;
    println!("mis à jour : {} → {}", env!("CARGO_PKG_VERSION"), release.tag);
    apply_hint();
    Ok(())
}

/// Rappel pour appliquer une modification au kiosk en cours.
fn apply_hint() {
    if cfg!(target_os = "linux") {
        println!("Appliquer : sudo systemctl restart ha-kiosk");
    }
}

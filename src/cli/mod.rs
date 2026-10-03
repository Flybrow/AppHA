//! Text commands: `config`, `setup`, `check`, `update`, and the help summary.

pub mod config_cmd;
mod prompt;
pub mod settings;
pub mod wizard;

use std::path::Path;

use anyhow::Result;

use crate::config::Config;
use crate::{health, tr, update};

/// Short help shown by `ha-kiosk` alone and `ha-kiosk --help`.
pub fn summary() -> String {
    let version = env!("CARGO_PKG_VERSION");
    tr!(
        format!(
            "HA Kiosk {version} — shows a Home Assistant dashboard full screen.

Usage: ha-kiosk <command>

  run                      start the kiosk
  setup                    configuration wizard
  config                   show all settings
  config <name> [value]    change a setting, e.g. ha-kiosk config rotation 180
  settings                 open the settings screen
  check                    test the connection to Home Assistant
  update                   install the latest version

  --config <file>          use another config file
"
        ),
        format!(
            "HA Kiosk {version} — affiche un dashboard Home Assistant en plein écran.

Utilisation : ha-kiosk <commande>

  run                      lance le kiosk
  setup                    assistant de configuration
  config                   affiche tous les réglages
  config <nom> [valeur]    modifie un réglage, ex. ha-kiosk config rotation 180
  settings                 ouvre l'écran de paramètres
  check                    teste la connexion à Home Assistant
  update                   installe la dernière version

  --config <fichier>       utilise un autre fichier de configuration
"
        )
    )
}

/// `ha-kiosk check`: config readable and Home Assistant reachable.
pub fn check(cfg: &Config, path: &Path) -> Result<()> {
    let ok = health::is_reachable(&cfg.url);
    let yes_no = |b: bool| if b { tr!("yes", "oui") } else { tr!("NO", "NON") };
    println!("{:<12} {}", tr!("config", "config"), path.display());
    println!("{:<12} {}", tr!("dashboard", "dashboard"), cfg.dashboard_url());
    println!("{:<12} {:?}", tr!("browser", "navigateur"), cfg.resolved_browser());
    println!("{:<12} {}", tr!("token", "jeton"), yes_no(!cfg.token.is_empty()));
    println!("{:<12} {}", tr!("reachable", "joignable"), yes_no(ok));
    if ok { Ok(()) } else { anyhow::bail!(tr!("Home Assistant unreachable", "Home Assistant injoignable")) }
}

/// `ha-kiosk update`; exit code 3 when already up to date.
pub fn update() -> Result<()> {
    let Some(release) = update::check()? else {
        println!("{} ({})", tr!("already up to date", "déjà à jour"), env!("CARGO_PKG_VERSION"));
        std::process::exit(3);
    };
    update::install(&release)?;
    println!("{} {} → {}", tr!("updated:", "mis à jour :"), env!("CARGO_PKG_VERSION"), release.tag);
    apply_hint();
    Ok(())
}

/// Reminder to apply a change to the running kiosk.
fn apply_hint() {
    if cfg!(target_os = "linux") {
        println!("{} sudo systemctl restart ha-kiosk", tr!("Apply:", "Appliquer :"));
    }
}

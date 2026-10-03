//! Démarrage automatique avec la machine (`autostart`).
//! Windows : clé `Run` de l'utilisateur (sans droits administrateur).
//! Linux : service systemd `ha-kiosk` (activation réservée à root).

use anyhow::Result;

/// Met le système en accord avec `enabled`.
pub fn apply(enabled: bool) -> Result<()> {
    imp::apply(enabled)
}

#[cfg(windows)]
mod imp {
    use std::process::Command;

    use anyhow::{Context, Result};

    use crate::process::run_quiet;

    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    const NAME: &str = "HA Kiosk";

    pub fn apply(enabled: bool) -> Result<()> {
        if enabled {
            let exe = std::env::current_exe().context("chemin de l'exécutable")?;
            let command = format!("\"{}\" run", exe.display());
            run_quiet(Command::new("reg").args(["add", KEY, "/v", NAME, "/t", "REG_SZ", "/d", &command, "/f"]))
                .context("inscription au démarrage de Windows")?;
        } else if is_registered() {
            run_quiet(Command::new("reg").args(["delete", KEY, "/v", NAME, "/f"])).context("retrait du démarrage de Windows")?;
        }
        Ok(())
    }

    fn is_registered() -> bool {
        run_quiet(Command::new("reg").args(["query", KEY, "/v", NAME])).is_ok()
    }
}

#[cfg(unix)]
mod imp {
    use std::process::Command;

    use anyhow::{Result, bail};

    use crate::process::run_quiet;

    const UNIT: &str = "ha-kiosk.service";

    pub fn apply(enabled: bool) -> Result<()> {
        // `is-enabled` répond sans root ; rien à faire si déjà conforme ou sans service installé.
        let state = run_quiet(Command::new("systemctl").args(["is-enabled", UNIT]))
            .map(|out| String::from_utf8_lossy(&out).trim().to_string())
            .unwrap_or_else(|_| "disabled".into());
        if state == "not-found" || (state == "enabled") == enabled {
            return Ok(());
        }
        let action = if enabled { "enable" } else { "disable" };
        if run_quiet(Command::new("systemctl").args([action, UNIT])).is_err() {
            let value = if enabled { "oui" } else { "non" };
            bail!("droits administrateur nécessaires : sudo ha-kiosk config autostart {value}");
        }
        Ok(())
    }
}

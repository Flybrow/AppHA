//! Starting with the machine (`autostart`).
//! Windows: the user's `Run` registry key (no administrator rights).
//! Linux: the `ha-kiosk` systemd service (enabling requires root).

use anyhow::Result;

/// Brings the system in line with `enabled`.
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
            let exe = std::env::current_exe().context("executable path")?;
            let command = format!("\"{}\" run", exe.display());
            run_quiet(Command::new("reg").args(["add", KEY, "/v", NAME, "/t", "REG_SZ", "/d", &command, "/f"]))
                .context("registering at Windows startup")?;
        } else if is_registered() {
            run_quiet(Command::new("reg").args(["delete", KEY, "/v", NAME, "/f"])).context("removing from Windows startup")?;
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
        // `is-enabled` works without root; nothing to do if already right or no service installed.
        let state = run_quiet(Command::new("systemctl").args(["is-enabled", UNIT]))
            .map(|out| String::from_utf8_lossy(&out).trim().to_string())
            .unwrap_or_else(|_| "disabled".into());
        if state == "not-found" || (state == "enabled") == enabled {
            return Ok(());
        }
        let action = if enabled { "enable" } else { "disable" };
        if run_quiet(Command::new("systemctl").args([action, UNIT])).is_err() {
            let value = if enabled { "yes" } else { "no" };
            bail!(
                "{} sudo ha-kiosk config autostart {value}",
                crate::tr!("administrator rights required:", "droits administrateur nécessaires :")
            );
        }
        Ok(())
    }
}

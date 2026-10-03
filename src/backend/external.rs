//! Navigateur externe défini par l'utilisateur (`command = ["cog", "{url}"]`).

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::config::Config;

pub fn command(cfg: &Config) -> Result<Command> {
    let url = cfg.dashboard_url();
    let mut args = cfg.command.iter().map(|a| a.replace("{url}", url.as_str()));
    let program = args.next().context("`command` est vide")?;
    let mut cmd = Command::new(&program);
    cmd.args(args);
    // cog (WPE WebKit) sait ignorer un certificat auto-signé.
    if cfg.insecure_tls && Path::new(&program).file_stem().is_some_and(|s| s == "cog") {
        cmd.arg("--ignore-tls-errors");
    }
    Ok(cmd)
}

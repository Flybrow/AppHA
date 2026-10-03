//! External browser defined by the user (`command = ["cog", "{url}"]`).

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::auth;
use crate::config::Config;

pub fn command(cfg: &Config) -> Result<Command> {
    let url = cfg.dashboard_url();
    let mut args = cfg.command.iter().map(|a| a.replace("{url}", url.as_str()));
    let program = args.next().context("`command` is empty")?;
    let mut cmd = Command::new(&program);
    cmd.args(args);
    // cog (WPE WebKit) can ignore a self-signed certificate.
    if cfg.insecure_tls && Path::new(&program).file_stem().is_some_and(|s| s == "cog") {
        cmd.arg("--ignore-tls-errors");
    }
    if !cfg.token.is_empty() {
        if auth::external_supports_token(&program) {
            cmd.args(auth::chromium_extension_args(&cfg.url, &cfg.token)?);
        } else {
            crate::warn!("{program} does not support token login: use browser = \"webview\" or Chromium");
        }
    }
    Ok(cmd)
}

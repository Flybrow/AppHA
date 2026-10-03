//! Configuration wizard (`ha-kiosk setup`): walks through the settings table,
//! for headless machines or SSH administration.

use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use serde_json::Value;
use url::Url;

use super::settings::{self, Setting};
use crate::config::Config;
use crate::{health, i18n, tr};

/// Sections (English, French) and settings asked, in order.
const STEPS: &[((&str, &str), &[&str])] = &[
    (("Language", "Langue"), &["language"]),
    (("Screen", "Écran"), &["rotation"]),
    (("Home Assistant", "Home Assistant"), &["url", "dashboard", "token", "insecure_tls"]),
    (("Browser", "Navigateur"), &["browser", "command"]),
    (("Display", "Affichage"), &["mode", "width", "height", "animations", "hide_cursor", "gpu"]),
    (("Stability and updates", "Stabilité et mises à jour"), &["autostart", "restart_every_hours", "max_memory_mb", "auto_update"]),
];

pub fn run(path: &Path) -> Result<()> {
    let mut cfg = Config::editable_json(path);
    println!("{} ({})", tr!("HA Kiosk configuration", "Configuration de HA Kiosk"), path.display());
    println!("{}", tr!("Enter keeps the value in brackets.", "Entrée = garder la valeur entre crochets."));

    for ((en, fr), names) in STEPS {
        println!("\n— {}", tr!(*en, *fr));
        for name in *names {
            // Evaluated before each question: depends on previous answers.
            if !relevant(name, &cfg) {
                continue;
            }
            let setting = settings::find(name)?;
            let value = if *name == "url" { ask_url(setting, &cfg)? } else { setting.prompt(&cfg)? };
            setting.set(&mut cfg, value)?;
            if *name == "language" {
                // The following questions use the chosen language.
                i18n::set(serde_json::from_value(cfg["language"].clone()).unwrap_or_default());
            }
        }
    }
    warn_token_unsupported(&cfg);

    Config::commit(cfg, path)?;
    println!("\n{} {}", tr!("Configuration saved in", "Configuration enregistrée dans"), path.display());
    super::apply_hint();
    Ok(())
}

/// Questions that only make sense depending on previous answers.
fn relevant(name: &str, cfg: &Value) -> bool {
    let is = |ptr: &str, v: &str| cfg.pointer(ptr).and_then(Value::as_str) == Some(v);
    match name {
        "insecure_tls" => cfg["url"].as_str().is_some_and(|u| u.starts_with("https")),
        "command" => is("/browser", "external"),
        "width" | "height" => !is("/window/mode", "fullscreen"),
        _ => true,
    }
}

/// Validated, then tested address (continues even if HA is unreachable).
fn ask_url(setting: &Setting, cfg: &Value) -> Result<Value> {
    loop {
        let value = setting.prompt(cfg)?;
        match value.as_str().map(Url::parse) {
            Some(Ok(url)) if matches!(url.scheme(), "http" | "https") => {
                print!("  {}… ", tr!("Testing the connection", "Test de connexion"));
                io::stdout().flush()?;
                let result = if health::is_reachable(&url) {
                    tr!("reachable ✔", "joignable ✔")
                } else {
                    tr!("unreachable (continuing anyway)", "injoignable (on continue quand même)")
                };
                println!("{result}");
                return Ok(value);
            }
            _ => println!("  {}", tr!("Invalid address: it must start with http:// or https://", "Adresse invalide : elle doit commencer par http:// ou https://")),
        }
    }
}

fn warn_token_unsupported(cfg: &Value) {
    let has_token = cfg["token"].as_str().is_some_and(|t| !t.is_empty());
    let program = cfg["command"].get(0).and_then(Value::as_str).unwrap_or("");
    if has_token && cfg["browser"] == "external" && !crate::auth::external_supports_token(program) {
        println!(
            "  {program}: {}",
            tr!("token login not supported (use the WebView or Chromium).", "connexion par jeton impossible (WebView ou Chromium requis).")
        );
    }
}

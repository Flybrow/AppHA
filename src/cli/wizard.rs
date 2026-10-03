//! Assistant de configuration (`ha-kiosk setup`) : enchaîne les questions de la
//! table des réglages, pour les machines sans écran ou administrées en SSH.

use std::io::{self, Write};
use std::path::Path;

use anyhow::Result;
use serde_json::Value;
use url::Url;

use super::settings::{self, Setting};
use crate::config::Config;
use crate::health;

/// Sections et réglages demandés, dans l'ordre.
const STEPS: &[(&str, &[&str])] = &[
    ("Écran", &["rotation"]),
    ("Home Assistant", &["url", "dashboard", "token", "insecure_tls"]),
    ("Navigateur", &["browser", "command"]),
    ("Affichage", &["mode", "width", "height", "animations", "hide_cursor", "gpu"]),
    ("Stabilité et mises à jour", &["autostart", "restart_every_hours", "max_memory_mb", "auto_update"]),
];

pub fn run(path: &Path) -> Result<()> {
    let mut cfg = Config::editable_json(path);
    println!("Configuration de ha-kiosk ({})", path.display());
    println!("Entrée = garder la valeur entre crochets.");

    for (section, names) in STEPS {
        println!("\n— {section}");
        for name in *names {
            // Évalué à chaque question : dépend des réponses précédentes.
            if !relevant(name, &cfg) {
                continue;
            }
            let setting = settings::find(name)?;
            let value = if *name == "url" { ask_url(setting, &cfg)? } else { setting.prompt(&cfg)? };
            setting.set(&mut cfg, value)?;
        }
    }
    warn_token_unsupported(&cfg);

    Config::commit(cfg, path)?;
    println!("\nConfiguration enregistrée dans {}", path.display());
    super::apply_hint();
    Ok(())
}

/// Questions qui n'ont de sens que selon les réponses précédentes.
fn relevant(name: &str, cfg: &Value) -> bool {
    let is = |ptr: &str, v: &str| cfg.pointer(ptr).and_then(Value::as_str) == Some(v);
    match name {
        "insecure_tls" => cfg["url"].as_str().is_some_and(|u| u.starts_with("https")),
        "command" => is("/browser", "external"),
        "width" | "height" => !is("/window/mode", "fullscreen"),
        _ => true,
    }
}

/// Adresse validée puis testée (on continue même si HA est injoignable).
fn ask_url(setting: &Setting, cfg: &Value) -> Result<Value> {
    loop {
        let value = setting.prompt(cfg)?;
        match value.as_str().map(Url::parse) {
            Some(Ok(url)) if matches!(url.scheme(), "http" | "https") => {
                print!("  Test de connexion… ");
                io::stdout().flush()?;
                println!("{}", if health::is_reachable(&url) { "joignable ✔" } else { "injoignable (on continue quand même)" });
                return Ok(value);
            }
            _ => println!("  Adresse invalide : elle doit commencer par http:// ou https://"),
        }
    }
}

fn warn_token_unsupported(cfg: &Value) {
    let has_token = cfg["token"].as_str().is_some_and(|t| !t.is_empty());
    let program = cfg["command"].get(0).and_then(Value::as_str).unwrap_or("");
    if has_token && cfg["browser"] == "external" && !crate::auth::external_supports_token(program) {
        println!("  Attention : {program} ne permet pas la connexion par jeton (WebView ou Chromium requis).");
    }
}

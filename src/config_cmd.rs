//! `ha-kiosk config [réglage] [valeur]` : lit ou modifie un seul réglage.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::config::Config;
use crate::wizard;

#[derive(Clone, Copy)]
enum Kind {
    Text,
    Secret,
    Bool,
    Number,
    Choice(&'static [&'static str]),
    List,
}

struct Setting {
    name: &'static str,
    pointer: &'static str,
    kind: Kind,
    help: &'static str,
}

const SETTINGS: &[Setting] = &[
    Setting { name: "url", pointer: "/url", kind: Kind::Text, help: "adresse de Home Assistant" },
    Setting { name: "dashboard", pointer: "/dashboard", kind: Kind::Text, help: "chemin du dashboard (vide = par défaut)" },
    Setting { name: "token", pointer: "/token", kind: Kind::Secret, help: "jeton d'accès longue durée" },
    Setting { name: "insecure_tls", pointer: "/insecure_tls", kind: Kind::Bool, help: "accepter un certificat auto-signé" },
    Setting { name: "browser", pointer: "/browser", kind: Kind::Choice(&["auto", "webview", "external"]), help: "navigateur" },
    Setting { name: "command", pointer: "/command", kind: Kind::List, help: "commande du navigateur externe ({url})" },
    Setting { name: "mode", pointer: "/window/mode", kind: Kind::Choice(&["fullscreen", "windowed", "borderless"]), help: "mode d'affichage" },
    Setting { name: "rotation", pointer: "/window/rotation", kind: Kind::Choice(&["0", "90", "180", "270"]), help: "rotation de l'écran (Linux)" },
    Setting { name: "width", pointer: "/window/width", kind: Kind::Number, help: "largeur en mode fenêtré" },
    Setting { name: "height", pointer: "/window/height", kind: Kind::Number, help: "hauteur en mode fenêtré" },
    Setting { name: "gpu", pointer: "/window/gpu", kind: Kind::Bool, help: "accélération matérielle (+~70 Mo)" },
    Setting { name: "auto_update", pointer: "/auto_update", kind: Kind::Bool, help: "mise à jour automatique" },
    Setting { name: "restart_every_hours", pointer: "/supervisor/restart_every_hours", kind: Kind::Number, help: "redémarrage préventif (0 = jamais)" },
    Setting { name: "max_memory_mb", pointer: "/supervisor/max_memory_mb", kind: Kind::Number, help: "limite RAM du navigateur (0 = aucune)" },
    Setting { name: "check_interval_secs", pointer: "/supervisor/check_interval_secs", kind: Kind::Number, help: "intervalle de surveillance" },
    Setting { name: "failures_before_down", pointer: "/supervisor/failures_before_down", kind: Kind::Number, help: "échecs avant coupure" },
    Setting { name: "reload_on_reconnect", pointer: "/supervisor/reload_on_reconnect", kind: Kind::Bool, help: "recharger au retour de HA" },
];

/// Liste des réglages pour l'aide de la ligne de commande.
pub fn help_text() -> String {
    let mut text = String::from("Réglages de `ha-kiosk config` :\n");
    for s in SETTINGS {
        let values = match s.kind {
            Kind::Choice(c) => format!(" ({})", c.join(" | ")),
            Kind::Bool => " (oui | non)".into(),
            _ => String::new(),
        };
        text.push_str(&format!("  {:<22} {}{values}\n", s.name, s.help));
    }
    text
}

pub fn run(path: &Path, name: Option<&str>, value: Option<&str>) -> Result<()> {
    let mut cfg = Config::load(path)
        .ok()
        .and_then(|c| serde_json::to_value(c).ok())
        .unwrap_or_else(Config::defaults_json);

    let Some(name) = name else {
        println!("{}", path.display());
        for s in SETTINGS {
            println!("  {:<22} {}", s.name, display(s, cfg.pointer(s.pointer)));
        }
        println!("\nModifier : ha-kiosk config <réglage> [valeur]");
        return Ok(());
    };
    let setting = SETTINGS
        .iter()
        .find(|s| s.name == name)
        .with_context(|| format!("réglage inconnu « {name} »\n\n{}", help_text()))?;

    let current = cfg.pointer(setting.pointer).cloned().unwrap_or(Value::Null);
    let new = match value {
        Some(v) => parse(setting, v)?,
        None => prompt(setting, &current)?,
    };
    *cfg.pointer_mut(setting.pointer).context("réglage absent")? = new;
    Config::from_json(cfg)?.save(path)?;
    println!("{} enregistré dans {}", setting.name, path.display());
    if cfg!(target_os = "linux") {
        println!("Appliquer : sudo systemctl restart ha-kiosk");
    }
    Ok(())
}

fn display(s: &Setting, v: Option<&Value>) -> String {
    match (s.kind, v) {
        (Kind::Secret, Some(Value::String(t))) if !t.is_empty() => format!("défini (…{})", &t[t.len().saturating_sub(4)..]),
        (Kind::Secret, _) => "(vide)".into(),
        (Kind::Bool, Some(Value::Bool(b))) => if *b { "oui" } else { "non" }.into(),
        (Kind::List, Some(Value::Array(a))) => a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "),
        (_, Some(Value::String(t))) if t.is_empty() => "(vide)".into(),
        (_, Some(Value::String(t))) => t.clone(),
        (_, Some(other)) => other.to_string(),
        (_, None) => "(non défini)".into(),
    }
}

fn parse(s: &Setting, raw: &str) -> Result<Value> {
    Ok(match s.kind {
        Kind::Text | Kind::Secret => json!(raw),
        Kind::Bool => match raw.to_lowercase().as_str() {
            "o" | "oui" | "y" | "yes" | "true" | "1" | "on" => json!(true),
            "n" | "non" | "no" | "false" | "0" | "off" => json!(false),
            _ => bail!("{} attend oui ou non", s.name),
        },
        Kind::Number => json!(raw.parse::<u64>().with_context(|| format!("{} attend un nombre", s.name))?),
        Kind::Choice(choices) => {
            if !choices.contains(&raw) {
                bail!("{} attend : {}", s.name, choices.join(" | "));
            }
            // Les choix numériques (rotation) sont stockés comme des nombres.
            raw.parse::<u64>().map(|n| json!(n)).unwrap_or_else(|_| json!(raw))
        }
        Kind::List => json!(raw.split_whitespace().collect::<Vec<_>>()),
    })
}

fn prompt(s: &Setting, current: &Value) -> Result<Value> {
    let question = format!("{} ({})", s.name, s.help);
    match s.kind {
        Kind::Secret => Ok(json!(wizard::ask_secret(&question, current.as_str().unwrap_or(""))?)),
        Kind::Bool => Ok(json!(wizard::ask_bool(&question, current.as_bool().unwrap_or(false))?)),
        Kind::Number => Ok(json!(wizard::ask_u64(&question, current.as_u64().unwrap_or(0))?)),
        Kind::Choice(choices) => {
            let options: Vec<(&str, &str)> = choices.iter().map(|c| (*c, "")).collect();
            let chosen = wizard::choose(&question, &options, &display(s, Some(current)))?;
            parse(s, &chosen)
        }
        Kind::Text | Kind::List => parse(s, &wizard::ask(&question, &display(s, Some(current)).replace("(vide)", ""))?),
    }
}

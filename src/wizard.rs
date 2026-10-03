//! Assistant de configuration en ligne de commande (`ha-kiosk setup`),
//! pour les machines sans écran ou administrées en SSH.

use std::io::{self, BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use url::Url;

use crate::config::Config;
use crate::health;

pub fn run(path: &Path) -> Result<()> {
    let mut cfg = Config::load(path)
        .ok()
        .and_then(|c| serde_json::to_value(c).ok())
        .unwrap_or_else(Config::defaults_json);
    println!("Configuration de ha-kiosk ({})", path.display());
    println!("Entrée = garder la valeur entre crochets.\n");

    println!("— Écran");
    cfg["window"]["rotation"] = json!(choose(
        "Rotation de l'écran",
        &[("0", "normale"), ("90", "90° (portrait)"), ("180", "180° (retournée)"), ("270", "270° (portrait inversé)")],
        &u64_at(&cfg, "/window/rotation").to_string(),
    )?
    .parse::<u16>()?);

    println!("
— Home Assistant");
    let url = loop {
        let raw = ask("Adresse (ex. https://192.168.1.10:8123)", str_at(&cfg, "/url"))?;
        match Url::parse(&raw) {
            Ok(u) if matches!(u.scheme(), "http" | "https") => break u,
            _ => println!("  Adresse invalide : elle doit commencer par http:// ou https://"),
        }
    };
    print!("  Test de connexion… ");
    io::stdout().flush()?;
    println!("{}", if health::is_reachable(&url) { "joignable ✔" } else { "injoignable (on continue quand même)" });
    cfg["url"] = json!(url.as_str());
    cfg["dashboard"] = json!(ask("Chemin du dashboard (vide = par défaut, ex. lovelace-kiosk/0)", str_at(&cfg, "/dashboard"))?);
    let token = ask_secret("Jeton longue durée (vide = connexion manuelle, - pour effacer)", str_at(&cfg, "/token"))?;
    cfg["token"] = json!(token);
    if url.scheme() == "https" {
        cfg["insecure_tls"] = json!(ask_bool("Certificat auto-signé à accepter", bool_at(&cfg, "/insecure_tls"))?);
    }

    println!("\n— Navigateur");
    let browser = choose(
        "Navigateur",
        &[
            ("auto", "cog si installé (le plus léger), sinon WebView intégrée"),
            ("webview", "WebView intégrée : paramètres à l'écran, connexion par jeton"),
            ("external", "commande personnalisée (Chromium, Firefox…)"),
        ],
        str_at(&cfg, "/browser"),
    )?;
    if browser == "external" {
        let current = cfg["command"].as_array().map(|a| join_args(a)).unwrap_or_default();
        let cmd = ask("Commande ({url} = adresse du dashboard)", &current)?;
        cfg["command"] = json!(cmd.split_whitespace().collect::<Vec<_>>());
    }
    cfg["browser"] = json!(browser);

    println!("\n— Affichage");
    cfg["window"]["mode"] = json!(choose(
        "Mode d'affichage",
        &[("fullscreen", "plein écran"), ("windowed", "fenêtré"), ("borderless", "fenêtré sans bordure")],
        str_at(&cfg, "/window/mode"),
    )?);
    if cfg["window"]["mode"] != "fullscreen" {
        cfg["window"]["width"] = json!(ask_u64("Largeur", u64_at(&cfg, "/window/width"))?);
        cfg["window"]["height"] = json!(ask_u64("Hauteur", u64_at(&cfg, "/window/height"))?);
    }
    cfg["window"]["gpu"] = json!(ask_bool("Accélération GPU (+~70 Mo, animations plus fluides)", bool_at(&cfg, "/window/gpu"))?);

    println!("\n— Stabilité et mises à jour");
    cfg["supervisor"]["restart_every_hours"] =
        json!(ask_u64("Redémarrage préventif toutes les N heures (0 = jamais)", u64_at(&cfg, "/supervisor/restart_every_hours"))?);
    cfg["supervisor"]["max_memory_mb"] =
        json!(ask_u64("Limite RAM du navigateur en Mo (0 = aucune, ex. 350 sur Pi 3)", u64_at(&cfg, "/supervisor/max_memory_mb"))?);
    cfg["auto_update"] = json!(ask_bool("Mise à jour automatique", bool_at(&cfg, "/auto_update"))?);

    let config = Config::from_json(cfg)?;
    config.save(path)?;
    println!("\nConfiguration enregistrée dans {}", path.display());
    Ok(())
}

fn ask(question: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("{question} : ");
    } else {
        print!("{question} [{default}] : ");
    }
    io::stdout().flush()?;
    let line = read_line()?;
    Ok(if line.is_empty() { default.to_string() } else { line })
}

/// Comme `ask`, sans réafficher le secret actuel.
fn ask_secret(question: &str, current: &str) -> Result<String> {
    let shown = if current.is_empty() { "" } else { "inchangé" };
    let answer = ask(question, shown)?;
    Ok(if answer == shown { current.to_string() } else if answer == "-" { String::new() } else { answer })
}

fn ask_bool(question: &str, default: bool) -> Result<bool> {
    loop {
        let answer = ask(&format!("{question} (o/n)"), if default { "o" } else { "n" })?;
        match answer.to_lowercase().as_str() {
            "o" | "oui" | "y" | "yes" => return Ok(true),
            "n" | "non" | "no" => return Ok(false),
            _ => println!("  Répondez o ou n."),
        }
    }
}

fn ask_u64(question: &str, default: u64) -> Result<u64> {
    loop {
        match ask(question, &default.to_string())?.parse() {
            Ok(n) => return Ok(n),
            Err(_) => println!("  Nombre attendu."),
        }
    }
}

/// Choix numéroté ; renvoie la valeur choisie.
fn choose(question: &str, options: &[(&str, &str)], default: &str) -> Result<String> {
    println!("{question} :");
    for (i, (value, help)) in options.iter().enumerate() {
        println!("  {}) {value} — {help}", i + 1);
    }
    let default_idx = options.iter().position(|(v, _)| *v == default).unwrap_or(0) + 1;
    loop {
        let answer = ask("Choix", &default_idx.to_string())?;
        if let Some((value, _)) = answer.parse::<usize>().ok().and_then(|n| options.get(n.wrapping_sub(1))) {
            return Ok(value.to_string());
        }
        if let Some((value, _)) = options.iter().find(|(v, _)| *v == answer) {
            return Ok(value.to_string());
        }
        println!("  Choix invalide.");
    }
}

fn read_line() -> Result<String> {
    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).context("lecture du terminal")? == 0 {
        bail!("entrée fermée : lancez l'assistant dans un terminal interactif");
    }
    Ok(line.trim().to_string())
}

fn join_args(args: &[Value]) -> String {
    args.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" ")
}

fn str_at<'a>(v: &'a Value, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(Value::as_str).unwrap_or("")
}

fn bool_at(v: &Value, ptr: &str) -> bool {
    v.pointer(ptr).and_then(Value::as_bool).unwrap_or(false)
}

fn u64_at(v: &Value, ptr: &str) -> u64 {
    v.pointer(ptr).and_then(Value::as_u64).unwrap_or(0)
}

//! Questions interactives dans le terminal, avec valeur par défaut.

use std::io::{self, BufRead, Write};

use anyhow::{Context, Result, bail};

/// Question libre ; Entrée garde `default`.
pub fn ask(question: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("{question} : ");
    } else {
        print!("{question} [{default}] : ");
    }
    io::stdout().flush()?;
    let line = read_line()?;
    Ok(if line.is_empty() { default.to_string() } else { line })
}

/// Comme `ask`, sans réafficher le secret actuel ; `-` l'efface.
pub fn ask_secret(question: &str, current: &str) -> Result<String> {
    let shown = if current.is_empty() { "" } else { "inchangé" };
    let answer = ask(&format!("{question} (- pour effacer)"), shown)?;
    Ok(match answer.as_str() {
        a if a == shown => current.to_string(),
        "-" => String::new(),
        _ => answer,
    })
}

pub fn ask_bool(question: &str, default: bool) -> Result<bool> {
    loop {
        let answer = ask(&format!("{question} (o/n)"), if default { "o" } else { "n" })?;
        match parse_bool(&answer) {
            Some(b) => return Ok(b),
            None => println!("  Répondez o ou n."),
        }
    }
}

pub fn ask_u64(question: &str, default: u64) -> Result<u64> {
    loop {
        match ask(question, &default.to_string())?.parse() {
            Ok(n) => return Ok(n),
            Err(_) => println!("  Nombre attendu."),
        }
    }
}

/// Choix numéroté parmi `(valeur, aide)` ; accepte la valeur ou son numéro.
pub fn choose(question: &str, options: &[(&str, &str)], default: &str) -> Result<String> {
    println!("{question} :");
    for (i, (value, help)) in options.iter().enumerate() {
        if help.is_empty() {
            println!("  {}) {value}", i + 1);
        } else {
            println!("  {}) {value} — {help}", i + 1);
        }
    }
    let default_idx = options.iter().position(|(v, _)| *v == default).unwrap_or(0) + 1;
    loop {
        let answer = ask("Choix", &default_idx.to_string())?;
        let by_value = options.iter().find(|(v, _)| *v == answer);
        let by_index = answer.parse::<usize>().ok().and_then(|n| options.get(n.wrapping_sub(1)));
        // La valeur prime : « 90 » est une rotation, pas le 90e choix.
        if let Some((value, _)) = by_value.or(by_index) {
            return Ok(value.to_string());
        }
        println!("  Choix invalide.");
    }
}

/// `oui` / `non` et leurs variantes ; `None` si illisible.
pub fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_lowercase().as_str() {
        "o" | "oui" | "y" | "yes" | "true" | "1" | "on" => Some(true),
        "n" | "non" | "no" | "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

fn read_line() -> Result<String> {
    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).context("lecture du terminal")? == 0 {
        bail!("entrée fermée : lancez la commande dans un terminal interactif");
    }
    Ok(line.trim().to_string())
}

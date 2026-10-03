//! Interactive terminal questions, with a default value.

use std::io::{self, BufRead, Write};

use anyhow::{Context, Result, bail};

use crate::tr;

/// Free question; Enter keeps `default`.
pub fn ask(question: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("{question}: ");
    } else {
        print!("{question} [{default}]: ");
    }
    io::stdout().flush()?;
    let line = read_line()?;
    Ok(if line.is_empty() { default.to_string() } else { line })
}

/// Like `ask`, without printing the current secret; `-` clears it.
pub fn ask_secret(question: &str, current: &str) -> Result<String> {
    let shown = if current.is_empty() { "" } else { tr!("unchanged", "inchangé") };
    let answer = ask(&format!("{question} {}", tr!("(- to clear)", "(- pour effacer)")), shown)?;
    Ok(match answer.as_str() {
        a if a == shown => current.to_string(),
        "-" => String::new(),
        _ => answer,
    })
}

pub fn ask_bool(question: &str, default: bool) -> Result<bool> {
    let (yes, no) = tr!(("y", "n"), ("o", "n"));
    loop {
        let answer = ask(&format!("{question} ({yes}/{no})"), if default { yes } else { no })?;
        match parse_bool(&answer) {
            Some(b) => return Ok(b),
            None => println!("  {}", tr!("Answer y or n.", "Répondez o ou n.")),
        }
    }
}

pub fn ask_u64(question: &str, default: u64) -> Result<u64> {
    loop {
        match ask(question, &default.to_string())?.parse() {
            Ok(n) => return Ok(n),
            Err(_) => println!("  {}", tr!("A number is expected.", "Nombre attendu.")),
        }
    }
}

/// Numbered choice among `(value, help)`; accepts the value or its number.
pub fn choose(question: &str, options: &[(&str, &str)], default: &str) -> Result<String> {
    println!("{question}:");
    for (i, (value, help)) in options.iter().enumerate() {
        if help.is_empty() {
            println!("  {}) {value}", i + 1);
        } else {
            println!("  {}) {value} — {help}", i + 1);
        }
    }
    let default_idx = options.iter().position(|(v, _)| *v == default).unwrap_or(0) + 1;
    loop {
        let answer = ask(tr!("Choice", "Choix"), &default_idx.to_string())?;
        let by_value = options.iter().find(|(v, _)| *v == answer);
        let by_index = answer.parse::<usize>().ok().and_then(|n| options.get(n.wrapping_sub(1)));
        // The value wins: "90" is a rotation, not the 90th choice.
        if let Some((value, _)) = by_value.or(by_index) {
            return Ok(value.to_string());
        }
        println!("  {}", tr!("Invalid choice.", "Choix invalide."));
    }
}

/// Yes/no in English or French; `None` when unreadable.
pub fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_lowercase().as_str() {
        "y" | "yes" | "o" | "oui" | "true" | "1" | "on" => Some(true),
        "n" | "no" | "non" | "false" | "0" | "off" => Some(false),
        _ => None,
    }
}

fn read_line() -> Result<String> {
    let mut line = String::new();
    if io::stdin().lock().read_line(&mut line).context("reading the terminal")? == 0 {
        bail!(tr!("input closed: run the command in an interactive terminal", "entrée fermée : lancez la commande dans un terminal interactif"));
    }
    Ok(line.trim().to_string())
}

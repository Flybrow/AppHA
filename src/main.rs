// Pas de console en release sous Windows : économise conhost (~2 Mo) et évite la fenêtre noire.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod auth;
mod backend;
mod config;
mod console;
mod display;
mod health;
mod log;
mod memory;
mod paths;
mod supervisor;
mod ui;
mod update;
mod wizard;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Chemin du fichier de configuration.
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Lance et supervise le navigateur (par défaut).
    Run,
    /// Ouvre directement la WebView intégrée, sans supervision.
    #[command(name = backend::WEBVIEW_SUBCOMMAND)]
    Webview,
    /// Vérifie la configuration et la connexion à Home Assistant.
    Check,
    /// Installe la dernière release GitHub si elle est plus récente
    /// (code de sortie 3 : déjà à jour).
    Update,
    /// Assistant de configuration en ligne de commande (questions / réponses).
    Setup,
    /// Ouvre l'écran de paramètres.
    #[command(name = backend::SETTINGS_SUBCOMMAND)]
    Settings {
        /// Premier lancement : pas de bouton Annuler.
        #[arg(long, hide = true)]
        first_run: bool,
    },
}

fn main() {
    let console = console::attach();
    if let Err(e) = run() {
        let text = format!("{e:#}");
        eprintln!("[ha-kiosk] ERREUR: {text}");
        if !console {
            console::error_dialog(&text);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    update::cleanup();
    let command = cli.command.unwrap_or(Cmd::Run);
    if let Cmd::Update = command {
        return manual_update();
    }
    let path = config::locate(cli.config)?;
    if let Cmd::Setup = command {
        return wizard::run(&path);
    }
    if let Cmd::Settings { first_run } = command {
        return ui::settings::run(&path, !first_run);
    }
    // Premier lancement : on demande la configuration avant de superviser.
    if matches!(command, Cmd::Run) && !path.is_file() {
        match backend::open_settings(&path, false)? {
            backend::SettingsOutcome::Saved | backend::SettingsOutcome::Updated => {}
            backend::SettingsOutcome::Quit => return Ok(()),
            backend::SettingsOutcome::Cancelled => anyhow::bail!("aucune configuration enregistrée"),
        }
    }
    let cfg = config::Config::load(&path)?;

    match command {
        Cmd::Run => supervisor::run(cfg, &path),
        Cmd::Webview => backend::webview::run(&cfg),
        Cmd::Check => check(&cfg, &path),
        Cmd::Settings { .. } | Cmd::Update | Cmd::Setup => unreachable!(),
    }
}

fn manual_update() -> Result<()> {
    let Some(release) = update::check()? else {
        println!("déjà à jour ({})", env!("CARGO_PKG_VERSION"));
        std::process::exit(3);
    };
    update::install(&release)?;
    println!("mis à jour : {} → {}", env!("CARGO_PKG_VERSION"), release.tag);
    Ok(())
}

fn check(cfg: &config::Config, path: &std::path::Path) -> Result<()> {
    println!("config      : {}", path.display());
    println!("dashboard   : {}", cfg.dashboard_url());
    println!("navigateur  : {:?}", cfg.resolved_browser());
    println!("jeton       : {}", if cfg.token.is_empty() { "non (connexion manuelle)" } else { "oui" });
    let ok = health::is_reachable(&cfg.url);
    println!("joignable   : {}", if ok { "oui" } else { "NON" });
    if ok { Ok(()) } else { anyhow::bail!("Home Assistant injoignable") }
}

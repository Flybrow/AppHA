// Pas de console en release sous Windows : économise conhost (~2 Mo) et évite la fenêtre noire.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod auth;
mod backend;
mod config;
mod config_cmd;
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
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, after_help = EXAMPLES)]
struct Cli {
    /// Chemin du fichier de configuration.
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Lance et supervise le kiosk (par défaut hors terminal : systemd, double-clic).
    Run,
    /// Affiche ou modifie un réglage : `config`, `config token`, `config rotation 180`.
    Config {
        /// Nom du réglage (sans nom : liste de tous les réglages).
        name: Option<String>,
        /// Nouvelle valeur (sans valeur : question interactive).
        value: Option<String>,
    },
    /// Interne : WebView intégrée sans supervision.
    #[command(name = backend::WEBVIEW_SUBCOMMAND, hide = true)]
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

const EXAMPLES: &str = "Exemples :
  ha-kiosk setup                 assistant complet (questions / réponses)
  ha-kiosk config                liste des réglages et de leurs valeurs
  ha-kiosk config token          modifie le jeton (question interactive)
  ha-kiosk config rotation 180   modifie directement un réglage
  ha-kiosk check                 teste la config et la connexion à HA
  ha-kiosk update                installe la dernière version
  ha-kiosk run                   lance le kiosk";

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
    let mut command = Cli::command().after_long_help(format!("{EXAMPLES}

{}", config_cmd::help_text()));
    let cli = Cli::from_arg_matches(&command.get_matches_mut()).map_err(|e| e.exit()).unwrap();
    // `ha-kiosk` seul tapé dans un terminal : aide. Lancé par systemd, cage ou un double-clic : kiosk.
    if cli.command.is_none() && cli.config.is_none() && launched_from_terminal() {
        command.print_long_help()?;
        return Ok(());
    }
    update::cleanup();
    let command = cli.command.unwrap_or(Cmd::Run);
    if let Cmd::Update = command {
        return manual_update();
    }
    let path = config::locate(cli.config)?;
    if let Cmd::Config { name, value } = &command {
        return config_cmd::run(&path, name.as_deref(), value.as_deref());
    }
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
        Cmd::Run => {
            backend::die_with_parent();
            supervisor::run(cfg, &path)
        }
        Cmd::Webview => backend::webview::run(&cfg),
        Cmd::Check => check(&cfg, &path),
        Cmd::Settings { .. } | Cmd::Update | Cmd::Setup | Cmd::Config { .. } => unreachable!(),
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

/// Vrai si un humain a lancé la commande dans un terminal (et non systemd, cage ou l'Explorateur).
fn launched_from_terminal() -> bool {
    use std::io::IsTerminal;
    if cfg!(windows) {
        // En release, la console n'existe que si un terminal parent l'a prêtée.
        return console::attached() && std::io::stdout().is_terminal();
    }
    std::env::var_os("INVOCATION_ID").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
        && std::io::stdin().is_terminal()
}

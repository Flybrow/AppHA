// No console in Windows release builds: saves conhost (~2 MB) and avoids the black window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod auth;
mod autostart;
mod backend;
mod cli;
mod config;
mod console;
mod display;
mod health;
mod i18n;
mod log;
mod memory;
mod paths;
mod process;
mod supervisor;
mod ui;
mod update;

use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};

#[derive(Parser)]
#[command(version, disable_help_subcommand = true)]
struct Cli {
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    Run,
    Setup,
    Config {
        name: Option<String>,
        value: Option<String>,
    },
    #[command(name = backend::SETTINGS_SUBCOMMAND)]
    Settings {
        /// First launch: no Cancel button.
        #[arg(long, hide = true)]
        first_run: bool,
    },
    Check,
    Update,
    /// Internal: built-in WebView without supervision.
    #[command(name = backend::WEBVIEW_SUBCOMMAND, hide = true)]
    Webview,
}

fn main() {
    let console = console::attach();
    if let Err(e) = run() {
        let text = format!("{e:#}");
        eprintln!("[ha-kiosk] {}: {text}", tr!("ERROR", "ERREUR"));
        if !console {
            console::error_dialog(&text);
        }
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let explicit_config = config_argument();
    if let Ok(path) = config::locate(explicit_config.clone()) {
        i18n::set(config::peek_language(&path));
    }
    let mut command = translated_command();
    let cli = Cli::from_arg_matches(&command.get_matches_mut()).map_err(|e| e.exit()).unwrap();
    // `ha-kiosk` typed alone in a terminal: help. Started by systemd, cage or a double-click: kiosk.
    if cli.command.is_none() && cli.config.is_none() && launched_from_terminal() {
        print!("{}", cli::summary());
        return Ok(());
    }
    update::cleanup();
    let command = cli.command.unwrap_or(Cmd::Run);
    if let Cmd::Update = command {
        return cli::update();
    }
    let path = config::locate(cli.config)?;
    match &command {
        Cmd::Config { name, value } => return cli::config_cmd::run(&path, name.as_deref(), value.as_deref()),
        Cmd::Setup => return cli::wizard::run(&path),
        Cmd::Settings { first_run } => return ui::settings::run(&path, !first_run),
        _ => {}
    }
    // First launch: ask for the configuration before supervising.
    if matches!(command, Cmd::Run) && !path.is_file() {
        match backend::open_settings(&path, false)? {
            backend::SettingsOutcome::Saved | backend::SettingsOutcome::Updated => {}
            backend::SettingsOutcome::Quit => return Ok(()),
            backend::SettingsOutcome::Cancelled => anyhow::bail!(tr!("no configuration saved", "aucune configuration enregistrée")),
        }
    }
    let cfg = config::Config::load(&path)?;
    i18n::set(cfg.language);

    match command {
        Cmd::Run => {
            backend::die_with_parent();
            supervisor::run(cfg, &path)
        }
        Cmd::Webview => backend::webview::run(&cfg),
        Cmd::Check => cli::check(&cfg, &path),
        Cmd::Settings { .. } | Cmd::Update | Cmd::Setup | Cmd::Config { .. } => unreachable!(),
    }
}

/// Clap definition with texts in the current language; `--help` shows the same
/// short summary as `ha-kiosk` alone.
fn translated_command() -> clap::Command {
    let about = |text: &'static str| move |c: clap::Command| c.about(text);
    Cli::command()
        .override_help(cli::summary())
        .mut_arg("config", |a| a.help(tr!("Config file to use", "Fichier de configuration à utiliser")))
        .mut_subcommand("run", about(tr!("Start the kiosk", "Lance le kiosk")))
        .mut_subcommand("setup", about(tr!("Configuration wizard", "Assistant de configuration")))
        .mut_subcommand("config", |c| {
            c.about(tr!("Show or change settings", "Affiche ou modifie les réglages"))
                .after_help(cli::settings::help_text())
                .mut_arg("name", |a| a.help(tr!("Setting name (none: list all)", "Nom du réglage (aucun : tout lister)")))
                .mut_arg("value", |a| a.help(tr!("New value (none: asked)", "Nouvelle valeur (aucune : demandée)")))
        })
        .mut_subcommand(backend::SETTINGS_SUBCOMMAND, about(tr!("Open the settings screen", "Ouvre l'écran de paramètres")))
        .mut_subcommand("check", about(tr!("Test the connection to Home Assistant", "Teste la connexion à Home Assistant")))
        .mut_subcommand("update", about(tr!("Install the latest version", "Installe la dernière version")))
}

/// `--config` / `-c` read before full parsing, to pick the help language.
fn config_argument() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy().into_owned();
        if arg == "-c" || arg == "--config" {
            return args.next().map(PathBuf::from);
        }
        if let Some(value) = arg.strip_prefix("--config=") {
            return Some(PathBuf::from(value));
        }
    }
    None
}

/// True if a human typed the command in a terminal (not systemd, cage or Explorer).
fn launched_from_terminal() -> bool {
    use std::io::IsTerminal;
    if cfg!(windows) {
        // In release builds, a console exists only if a parent terminal lent one.
        return console::attached() && std::io::stdout().is_terminal();
    }
    std::env::var_os("INVOCATION_ID").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
        && std::io::stdin().is_terminal()
}

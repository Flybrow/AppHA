//! Chargement et validation de `config.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::paths;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    /// `external` si `cog` est disponible (Linux), sinon `webview`.
    Auto,
    /// WebView système intégrée (WebView2 sous Windows, WebKitGTK sous Linux).
    Webview,
    /// Commande externe définie dans `command` (cog, chromium, firefox...).
    External,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    /// Plein écran sans bordure, sur tout l'écran.
    #[default]
    Fullscreen,
    /// Fenêtre classique avec barre de titre.
    Windowed,
    /// Fenêtre de taille `width` × `height`, sans barre de titre ni bordure.
    Borderless,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct WindowConfig {
    pub mode: WindowMode,
    pub width: u32,
    pub height: u32,
    /// Accélération matérielle de la WebView. Désactivée, elle économise ~70 Mo
    /// (process GPU) au prix d'un rendu logiciel des animations.
    pub gpu: bool,
    /// Ancien réglage (`fullscreen = false`), converti en `mode` au chargement.
    #[serde(skip_serializing)]
    fullscreen: Option<bool>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { mode: WindowMode::Fullscreen, width: 1280, height: 800, gpu: false, fullscreen: None }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SupervisorConfig {
    /// Intervalle entre deux vérifications (santé HA, mémoire, process).
    pub check_interval_secs: u64,
    /// Redémarrage préventif du navigateur (0 = désactivé).
    pub restart_every_hours: u64,
    /// Seuil RAM de l'arbre de process du navigateur (0 = désactivé).
    pub max_memory_mb: u64,
    /// Relance le navigateur quand HA redevient joignable après une coupure.
    pub reload_on_reconnect: bool,
    /// Nombre d'échecs consécutifs avant de considérer HA comme injoignable.
    pub failures_before_down: u32,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 10,
            restart_every_hours: 24,
            max_memory_mb: 0,
            reload_on_reconnect: true,
            failures_before_down: 3,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub url: Url,
    #[serde(default)]
    pub dashboard: String,
    #[serde(default)]
    pub token: String,
    #[serde(default = "default_browser")]
    pub browser: Browser,
    #[serde(default = "default_command")]
    pub command: Vec<String>,
    #[serde(default)]
    pub insecure_tls: bool,
    /// Mise à jour automatique depuis les releases GitHub (superviseur sous Windows,
    /// minuteur systemd sous Linux).
    #[serde(default = "default_true")]
    pub auto_update: bool,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub supervisor: SupervisorConfig,
}

fn default_true() -> bool {
    true
}

fn default_browser() -> Browser {
    Browser::Auto
}

fn default_command() -> Vec<String> {
    vec!["cog".into(), "{url}".into()]
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("lecture de {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&raw).with_context(|| format!("analyse de {}", path.display()))?;
        let mut cfg = cfg;
        if cfg.window.fullscreen.take() == Some(false) && cfg.window.mode == WindowMode::Fullscreen {
            cfg.window.mode = WindowMode::Windowed;
        }
        cfg.validate()?;
        Ok(cfg)
    }

    /// Config envoyée par l'écran de paramètres (JSON), validée.
    pub fn from_json(json: serde_json::Value) -> Result<Self> {
        let cfg: Config = serde_json::from_value(json).context("paramètres invalides")?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Écrit la config, en créant le dossier parent si besoin.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("création de {}", dir.display()))?;
        }
        let raw = toml::to_string_pretty(self).context("sérialisation de la config")?;
        std::fs::write(path, raw).with_context(|| format!("écriture de {}", path.display()))
    }

    /// Valeurs par défaut proposées au premier lancement (aucune config).
    pub fn defaults_json() -> serde_json::Value {
        serde_json::json!({
            "url": "http://homeassistant.local:8123",
            "dashboard": "",
            "token": "",
            "browser": default_browser(),
            "command": default_command(),
            "insecure_tls": false,
            "auto_update": true,
            "window": WindowConfig::default(),
            "supervisor": SupervisorConfig::default(),
        })
    }

    fn validate(&self) -> Result<()> {
        if !matches!(self.url.scheme(), "http" | "https") {
            bail!("`url` doit être en http:// ou https://");
        }
        if self.browser == Browser::External && self.command.is_empty() {
            bail!("`command` est vide alors que browser = \"external\"");
        }
        Ok(())
    }

    /// URL complète du dashboard à afficher.
    pub fn dashboard_url(&self) -> Url {
        let path = self.dashboard.trim_matches('/');
        if path.is_empty() { self.url.clone() } else { self.url.join(path).unwrap_or_else(|_| self.url.clone()) }
    }

    /// Backend réellement utilisé une fois `auto` résolu.
    pub fn resolved_browser(&self) -> Browser {
        match self.browser {
            Browser::Auto if cfg!(target_os = "linux") && paths::in_path("cog") => Browser::External,
            Browser::Auto => Browser::Webview,
            other => other,
        }
    }
}

/// Fichier de config à utiliser : argument explicite, sinon à côté de l'exécutable,
/// sinon dans le dossier de config utilisateur. S'il n'existe nulle part, renvoie
/// l'emplacement où le créer (dossier de config utilisateur).
pub fn locate(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    let candidates: Vec<PathBuf> =
        [paths::exe_dir(), paths::config_dir()].into_iter().flatten().map(|d| d.join("config.toml")).collect();
    candidates
        .iter()
        .find(|p| p.is_file())
        .or(candidates.last())
        .cloned()
        .context("aucun dossier de configuration disponible — utilisez --config")
}

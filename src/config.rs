//! Chargement et validation de `config.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::paths;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    /// `webview` : seul backend qui gère à la fois le jeton et l'écran de paramètres.
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
    /// Rotation de l'écran en degrés (0, 90, 180, 270), appliquée sous Wayland/cage.
    pub rotation: u16,
    /// Animations et transitions des pages. Désactivées, le frontend HA repeint
    /// beaucoup moins : gros gain de CPU sur les petites machines (défaut : non sous Linux).
    pub animations: bool,
    /// Masque le curseur de la souris (écran tactile).
    pub hide_cursor: bool,
    /// Ancien réglage (`fullscreen = false`), converti en `mode` au chargement.
    #[serde(skip_serializing)]
    fullscreen: Option<bool>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { mode: WindowMode::Fullscreen, width: 1280, height: 800, gpu: false, rotation: 0, animations: !cfg!(target_os = "linux"), hide_cursor: false, fullscreen: None }
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
    /// Mise à jour automatique depuis les releases GitHub, par le superviseur.
    #[serde(default = "default_true")]
    pub auto_update: bool,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub supervisor: SupervisorConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: Url::parse("http://homeassistant.local:8123").expect("URL par défaut valide"),
            dashboard: String::new(),
            token: String::new(),
            browser: default_browser(),
            command: default_command(),
            insecure_tls: false,
            auto_update: true,
            window: WindowConfig::default(),
            supervisor: SupervisorConfig::default(),
        }
    }
}

/// Fusion récursive : les valeurs de `patch` remplacent celles de `base`.
fn merge(base: &mut serde_json::Value, patch: serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(base), serde_json::Value::Object(patch)) => {
            for (key, value) in patch {
                merge(base.entry(key).or_insert(serde_json::Value::Null), value);
            }
        }
        (base, patch) => *base = patch,
    }
}

/// Reprend propriétaire et droits de `original` (s'il existe) sur `target`.
#[cfg(unix)]
fn copy_ownership(original: &Path, target: &Path) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let Ok(meta) = std::fs::metadata(original) else {
        // Nouveau fichier : il contient le jeton, lisible seulement par son propriétaire.
        return Ok(std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o600))?);
    };
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(meta.mode() & 0o7777))?;
    // Échoue sans root si le propriétaire diffère : sans conséquence, c'est alors l'utilisateur courant.
    let _ = std::os::unix::fs::chown(target, Some(meta.uid()), Some(meta.gid()));
    Ok(())
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
        let mut cfg: Config =
            toml::from_str(&raw).with_context(|| format!("analyse de {}", path.display()))?;
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

    /// Écrit la config de façon atomique (fichier temporaire puis renommage) : une
    /// coupure de courant ne laisse jamais un fichier tronqué. Sous Unix, le
    /// propriétaire et les droits du fichier existant sont conservés (jeton protégé).
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("création de {}", dir.display()))?;
        }
        let raw = toml::to_string_pretty(self).context("sérialisation de la config")?;
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, raw).with_context(|| format!("écriture de {}", tmp.display()))?;
        #[cfg(unix)]
        copy_ownership(path, &tmp)?;
        std::fs::rename(&tmp, path).with_context(|| format!("remplacement de {}", path.display()))
    }

    /// Config à éditer (assistant, `config`, écran de paramètres) : valeurs par
    /// défaut complétées par le fichier. Un fichier invalide est signalé puis
    /// conservé autant que possible, au lieu d'être silencieusement remplacé.
    pub fn editable_json(path: &Path) -> serde_json::Value {
        let mut json = serde_json::to_value(Config::default()).unwrap_or_default();
        if !path.is_file() {
            return json;
        }
        let file = std::fs::read_to_string(path)
            .map_err(anyhow::Error::from)
            .and_then(|raw| toml::from_str::<serde_json::Value>(&raw).map_err(Into::into));
        match file {
            Ok(values) => merge(&mut json, values),
            Err(e) => crate::warn!("{} illisible ({e}) : valeurs par défaut proposées", path.display()),
        }
        if let Err(e) = Config::from_json(json.clone()) {
            crate::warn!("configuration actuelle invalide : {e:#}");
        }
        json
    }

    fn validate(&self) -> Result<()> {
        if !matches!(self.url.scheme(), "http" | "https") {
            bail!("`url` doit être en http:// ou https://");
        }
        if ![0, 90, 180, 270].contains(&self.window.rotation) {
            bail!("`window.rotation` doit valoir 0, 90, 180 ou 270");
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
            Browser::Auto => Browser::Webview,
            other => other,
        }
    }
}

/// Fichier de config à utiliser : argument explicite, sinon à côté de l'exécutable,
/// sinon dans /etc/ha-kiosk (Linux), sinon dans le dossier de config utilisateur. S'il n'existe nulle part, renvoie
/// l'emplacement où le créer (dossier de config utilisateur).
pub fn locate(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    // /etc/ha-kiosk : emplacement de l'installation Linux (service systemd).
    let system = cfg!(target_os = "linux").then(|| PathBuf::from("/etc/ha-kiosk"));
    let candidates: Vec<PathBuf> =
        [paths::exe_dir(), system, paths::config_dir()].into_iter().flatten().map(|d| d.join("config.toml")).collect();
    candidates
        .iter()
        .find(|p| p.is_file())
        .or(candidates.last())
        .cloned()
        .context("aucun dossier de configuration disponible — utilisez --config")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_keeps_defaults_and_overrides() {
        let mut base = json!({"a": 1, "w": {"x": 1, "y": 2}});
        merge(&mut base, json!({"w": {"y": 3}, "b": true}));
        assert_eq!(base, json!({"a": 1, "w": {"x": 1, "y": 3}, "b": true}));
    }

    #[test]
    fn default_round_trips_through_json() {
        let json = serde_json::to_value(Config::default()).unwrap();
        assert!(Config::from_json(json).is_ok());
    }

    #[test]
    fn validation() {
        let mut json = serde_json::to_value(Config::default()).unwrap();
        json["window"]["rotation"] = json!(45);
        assert!(Config::from_json(json.clone()).is_err());
        json["window"]["rotation"] = json!(180);
        json["url"] = json!("ftp://ha");
        assert!(Config::from_json(json.clone()).is_err());
        json["url"] = json!("https://ha:8123");
        json["browser"] = json!("external");
        json["command"] = json!([]);
        assert!(Config::from_json(json).is_err());
    }

    #[test]
    fn dashboard_url() {
        let mut cfg = Config { url: Url::parse("https://ha:8123/").unwrap(), ..Config::default() };
        assert_eq!(cfg.dashboard_url().as_str(), "https://ha:8123/");
        cfg.dashboard = "/lovelace-kiosk/0/".into();
        assert_eq!(cfg.dashboard_url().as_str(), "https://ha:8123/lovelace-kiosk/0");
    }

    #[test]
    fn legacy_fullscreen_false_becomes_windowed() {
        let dir = std::env::temp_dir().join(format!("hk-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "url = \"http://ha\"
[window]
fullscreen = false
").unwrap();
        assert_eq!(Config::load(&path).unwrap().window.mode, WindowMode::Windowed);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn save_is_readable_back() {
        let dir = std::env::temp_dir().join(format!("hk-save-{}", std::process::id()));
        let path = dir.join("config.toml");
        let cfg = Config { token: "secret".into(), ..Config::default() };
        cfg.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap().token, "secret");
        assert!(!path.with_extension("toml.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}

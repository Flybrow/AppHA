//! Loading, validation and saving of `config.toml`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::i18n::Language;
use crate::{paths, tr};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Browser {
    /// `webview`: the only backend with both token login and on-screen settings.
    Auto,
    /// Built-in system WebView (WebView2 on Windows, WebKitGTK on Linux).
    Webview,
    /// External command from `command` (cog, chromium, firefox…).
    External,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    /// Borderless, covering the whole screen.
    #[default]
    Fullscreen,
    /// Regular window with a title bar.
    Windowed,
    /// `width` × `height` window without title bar or border.
    Borderless,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct WindowConfig {
    pub mode: WindowMode,
    pub width: u32,
    pub height: u32,
    /// Hardware acceleration. Off, it saves ~70 MB (GPU process) on Windows.
    pub gpu: bool,
    /// Screen rotation in degrees (0, 90, 180, 270), applied under Wayland/cage.
    pub rotation: u16,
    /// Page animations and transitions. Off, the HA frontend stops repainting
    /// constantly: a big CPU saving on small machines (default: off on Linux).
    pub animations: bool,
    /// Hides the mouse cursor (touch screens).
    pub hide_cursor: bool,
    /// Legacy setting (`fullscreen = false`), converted to `mode` on load.
    #[serde(skip_serializing)]
    fullscreen: Option<bool>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            mode: WindowMode::Fullscreen,
            width: 1280,
            height: 800,
            gpu: false,
            rotation: 0,
            animations: !cfg!(target_os = "linux"),
            hide_cursor: false,
            fullscreen: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SupervisorConfig {
    /// Interval between checks (HA health, memory, process).
    pub check_interval_secs: u64,
    /// Preventive browser restart (0 = disabled).
    pub restart_every_hours: u64,
    /// RAM limit for the browser process tree (0 = disabled).
    pub max_memory_mb: u64,
    /// Reloads the browser when HA becomes reachable again after an outage.
    pub reload_on_reconnect: bool,
    /// Consecutive failures before HA is considered down.
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
    /// Interface language.
    #[serde(default)]
    pub language: Language,
    #[serde(default = "default_browser")]
    pub browser: Browser,
    #[serde(default = "default_command")]
    pub command: Vec<String>,
    #[serde(default)]
    pub insecure_tls: bool,
    /// Automatic updates from GitHub releases, done by the supervisor.
    #[serde(default = "default_true")]
    pub auto_update: bool,
    /// Starts with the machine.
    #[serde(default = "default_true")]
    pub autostart: bool,
    #[serde(default)]
    pub window: WindowConfig,
    #[serde(default)]
    pub supervisor: SupervisorConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: Url::parse("http://homeassistant.local:8123").expect("valid default URL"),
            dashboard: String::new(),
            token: String::new(),
            language: Language::Auto,
            browser: default_browser(),
            command: default_command(),
            insecure_tls: false,
            auto_update: true,
            autostart: true,
            window: WindowConfig::default(),
            supervisor: SupervisorConfig::default(),
        }
    }
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
        let raw = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let mut cfg: Config = toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        if cfg.window.fullscreen.take() == Some(false) && cfg.window.mode == WindowMode::Fullscreen {
            cfg.window.mode = WindowMode::Windowed;
        }
        cfg.validate()?;
        Ok(cfg)
    }

    /// Config sent by the settings screen or the CLI (JSON), validated.
    pub fn from_json(json: serde_json::Value) -> Result<Self> {
        let cfg: Config = serde_json::from_value(json).context(tr!("invalid settings", "paramètres invalides"))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validates, saves, then applies what affects the system (autostart).
    /// Single entry point for every change.
    pub fn commit(json: serde_json::Value, path: &Path) -> Result<Config> {
        let cfg = Config::from_json(json)?;
        cfg.save(path)?;
        crate::autostart::apply(cfg.autostart)
            .context(tr!("settings saved, but autostart not applied", "configuration enregistrée, mais démarrage automatique non appliqué"))?;
        Ok(cfg)
    }

    /// Atomic write (temporary file, then rename): a power cut never leaves a
    /// truncated file. On Unix, owner and permissions of the existing file are kept
    /// (the token stays protected). Falls back to a direct write when only the
    /// file, not its directory, is writable.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let raw = toml::to_string_pretty(self).context("serializing the config")?;
        let tmp = path.with_extension("toml.tmp");
        match std::fs::write(&tmp, &raw) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && path.is_file() => {
                return std::fs::write(path, raw).with_context(|| format!("writing {}", path.display()));
            }
            Err(e) => return Err(e).with_context(|| format!("writing {}", tmp.display())),
        }
        #[cfg(unix)]
        copy_ownership(path, &tmp)?;
        std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
    }

    /// Config to edit (wizard, `config`, settings screen): defaults completed by
    /// the file. An invalid file is reported and kept as far as possible instead
    /// of being silently replaced.
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
            Err(e) => crate::warn!("{} unreadable ({e}): showing defaults", path.display()),
        }
        if let Err(e) = Config::from_json(json.clone()) {
            crate::warn!("current config is invalid: {e:#}");
        }
        json
    }

    fn validate(&self) -> Result<()> {
        if !matches!(self.url.scheme(), "http" | "https") {
            bail!(tr!("`url` must start with http:// or https://", "`url` doit commencer par http:// ou https://"));
        }
        if ![0, 90, 180, 270].contains(&self.window.rotation) {
            bail!(tr!("`rotation` must be 0, 90, 180 or 270", "`rotation` doit valoir 0, 90, 180 ou 270"));
        }
        if self.browser == Browser::External && self.command.is_empty() {
            bail!(tr!("`command` is empty while browser = \"external\"", "`command` est vide alors que browser = \"external\""));
        }
        Ok(())
    }

    /// Full URL of the dashboard to display.
    pub fn dashboard_url(&self) -> Url {
        let path = self.dashboard.trim_matches('/');
        if path.is_empty() { self.url.clone() } else { self.url.join(path).unwrap_or_else(|_| self.url.clone()) }
    }

    /// Backend actually used once `auto` is resolved.
    pub fn resolved_browser(&self) -> Browser {
        match self.browser {
            Browser::Auto => Browser::Webview,
            other => other,
        }
    }
}

/// Recursive merge: values from `patch` replace those in `base`.
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

/// Copies owner and permissions of `original` (if it exists) onto `target`.
#[cfg(unix)]
fn copy_ownership(original: &Path, target: &Path) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let Ok(meta) = std::fs::metadata(original) else {
        // New file: it holds the token, readable by its owner only.
        return Ok(std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o600))?);
    };
    std::fs::set_permissions(target, std::fs::Permissions::from_mode(meta.mode() & 0o7777))?;
    // Fails without root when the owner differs: harmless, the owner is then the current user.
    let _ = std::os::unix::fs::chown(target, Some(meta.uid()), Some(meta.gid()));
    Ok(())
}

/// Config file to use: explicit argument, else next to the executable, else
/// /etc/ha-kiosk (Linux), else the user config directory. If none exists, returns
/// where to create it (user config directory).
pub fn locate(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    // /etc/ha-kiosk: Linux installation (systemd service).
    let system = cfg!(target_os = "linux").then(|| PathBuf::from("/etc/ha-kiosk"));
    let candidates: Vec<PathBuf> =
        [paths::exe_dir(), system, paths::config_dir()].into_iter().flatten().map(|d| d.join("config.toml")).collect();
    candidates
        .iter()
        .find(|p| p.is_file())
        .or(candidates.last())
        .cloned()
        .context("no config directory available — use --config")
}

/// Reads only the `language` setting, before the full config is loaded
/// (help, wizard), so a broken config does not prevent picking the language.
pub fn peek_language(path: &Path) -> Language {
    #[derive(Deserialize)]
    struct Peek {
        #[serde(default)]
        language: Language,
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| toml::from_str::<Peek>(&raw).ok())
        .map_or(Language::Auto, |p| p.language)
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

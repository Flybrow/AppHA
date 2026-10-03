//! Connexion automatique au frontend HA avec un jeton longue durée.
//!
//! Le frontend lit ses identifiants dans `localStorage["hassTokens"]`. On y place
//! le jeton avec une expiration lointaine : aucun écran de connexion, aucun refresh.

use std::path::Path;

use anyhow::{Context, Result};
use url::Url;

/// Script exécuté avant chaque chargement de page (uniquement sur l'origine HA).
pub fn init_script(ha_url: &Url, token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }
    let origin = ha_url.origin().ascii_serialization();
    let tokens = serde_json::json!({
        "access_token": token,
        "token_type": "Bearer",
        "expires_in": 1800,
        "hassUrl": origin,
        "clientId": format!("{origin}/"),
        "expires": 9_999_999_999_999u64,
        "refresh_token": "",
    });
    let origin_js = serde_json::to_string(&origin).ok()?;
    let tokens_js = serde_json::to_string(&tokens.to_string()).ok()?;
    Some(format!(
        "if (location.origin === {origin_js}) {{ try {{ localStorage.setItem('hassTokens', {tokens_js}); }} catch (e) {{}} }}"
    ))
}

/// Navigateurs externes de la famille Chromium : le jeton y est injecté par une extension.
const CHROMIUM: &[&str] = &["chromium", "chromium-browser", "chrome", "google-chrome", "google-chrome-stable", "msedge", "brave", "brave-browser"];

/// Vrai si le navigateur externe `program` peut recevoir le jeton.
pub fn external_supports_token(program: &str) -> bool {
    Path::new(program).file_stem().and_then(|s| s.to_str()).is_some_and(|s| CHROMIUM.contains(&s))
}

/// Écrit une extension Chromium (script au démarrage de chaque page, filtré sur
/// l'origine HA) et renvoie les arguments qui la chargent.
pub fn chromium_extension_args(ha_url: &Url, token: &str) -> Result<Vec<String>> {
    let Some(script) = init_script(ha_url, token) else { return Ok(Vec::new()) };
    let dir = crate::paths::data_dir().context("dossier de données introuvable")?.join("chromium-auth");
    std::fs::create_dir_all(&dir).with_context(|| format!("création de {}", dir.display()))?;
    let manifest = serde_json::json!({
        "manifest_version": 3,
        "name": "ha-kiosk auth",
        "version": "1.0",
        "content_scripts": [{ "matches": ["<all_urls>"], "js": ["auth.js"], "run_at": "document_start" }],
    });
    std::fs::write(dir.join("manifest.json"), manifest.to_string())?;
    let script_path = dir.join("auth.js");
    std::fs::write(&script_path, script)?;
    #[cfg(unix)]
    {
        // Contient le jeton.
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(vec![
        format!("--load-extension={}", dir.display()),
        // Chrome 137+ ignore --load-extension sans ce réglage.
        "--disable-features=DisableLoadExtensionCommandLineSwitch".into(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_only_for_ha_origin_and_with_token() {
        let url = Url::parse("https://192.168.1.10:8123/").unwrap();
        assert!(init_script(&url, "").is_none());
        let script = init_script(&url, "abc").unwrap();
        assert!(script.contains("\"https://192.168.1.10:8123\""));
        assert!(script.contains("abc"));
    }

    #[test]
    fn chromium_detection() {
        assert!(external_supports_token("/usr/bin/chromium"));
        assert!(external_supports_token("msedge.exe"));
        assert!(!external_supports_token("cog"));
        assert!(!external_supports_token("firefox"));
    }
}

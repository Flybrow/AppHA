//! Automatic login to the HA frontend with a long-lived access token.
//!
//! The frontend reads its credentials from `localStorage["hassTokens"]`. The token
//! is stored there with a far expiry: no login screen, no refresh.

use std::path::Path;

use anyhow::{Context, Result};
use url::Url;

/// Script run before each page load (on the HA origin only).
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

/// Chromium-family external browsers: the token is injected by an extension.
const CHROMIUM: &[&str] = &["chromium", "chromium-browser", "chrome", "google-chrome", "google-chrome-stable", "msedge", "brave", "brave-browser"];

/// True if the external browser `program` can receive the token.
pub fn external_supports_token(program: &str) -> bool {
    Path::new(program).file_stem().and_then(|s| s.to_str()).is_some_and(|s| CHROMIUM.contains(&s))
}

/// Writes a Chromium extension (script at the start of each page, filtered on the
/// HA origin) and returns the arguments that load it.
pub fn chromium_extension_args(ha_url: &Url, token: &str) -> Result<Vec<String>> {
    let Some(script) = init_script(ha_url, token) else { return Ok(Vec::new()) };
    let dir = crate::paths::data_dir().context("data directory not found")?.join("chromium-auth");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
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
        // Holds the token.
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(vec![
        format!("--load-extension={}", dir.display()),
        // Chrome 137+ ignores --load-extension without this.
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

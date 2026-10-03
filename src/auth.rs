//! Connexion automatique au frontend HA avec un jeton longue durée.
//!
//! Le frontend lit ses identifiants dans `localStorage["hassTokens"]`. On y place
//! le jeton avec une expiration lointaine : aucun écran de connexion, aucun refresh.

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

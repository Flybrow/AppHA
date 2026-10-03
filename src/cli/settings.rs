//! Table des réglages modifiables en ligne de commande, partagée par
//! `ha-kiosk config` et l'assistant `ha-kiosk setup`.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::prompt;

#[derive(Clone, Copy)]
pub enum Kind {
    Text,
    Secret,
    Bool,
    Number,
    /// `(valeur, aide)` ; les valeurs numériques sont stockées comme nombres.
    Choice(&'static [(&'static str, &'static str)]),
    /// Arguments séparés par des espaces.
    List,
}

pub struct Setting {
    pub name: &'static str,
    pointer: &'static str,
    kind: Kind,
    help: &'static str,
}

const fn setting(name: &'static str, pointer: &'static str, kind: Kind, help: &'static str) -> Setting {
    Setting { name, pointer, kind, help }
}

pub const SETTINGS: &[Setting] = &[
    setting("rotation", "/window/rotation", Kind::Choice(&[("0", "normale"), ("90", "portrait"), ("180", "retournée"), ("270", "portrait inversé")]), "rotation de l'écran (Linux)"),
    setting("url", "/url", Kind::Text, "adresse de Home Assistant, ex. https://192.168.1.10:8123"),
    setting("dashboard", "/dashboard", Kind::Text, "chemin du dashboard (vide = par défaut, ex. lovelace-kiosk/0)"),
    setting("token", "/token", Kind::Secret, "jeton d'accès longue durée (vide = connexion manuelle)"),
    setting("insecure_tls", "/insecure_tls", Kind::Bool, "accepter un certificat auto-signé"),
    setting("browser", "/browser", Kind::Choice(&[("auto", "WebView intégrée (recommandé)"), ("webview", "WebView intégrée"), ("external", "commande : Chromium (jeton OK), cog, Firefox (sans jeton)")]), "navigateur"),
    setting("command", "/command", Kind::List, "commande du navigateur externe ({url} = dashboard)"),
    setting("mode", "/window/mode", Kind::Choice(&[("fullscreen", "plein écran"), ("windowed", "fenêtré"), ("borderless", "fenêtré sans bordure")]), "mode d'affichage"),
    setting("width", "/window/width", Kind::Number, "largeur en mode fenêtré"),
    setting("height", "/window/height", Kind::Number, "hauteur en mode fenêtré"),
    setting("animations", "/window/animations", Kind::Bool, "animations des pages (non = bien plus fluide sur Pi)"),
    setting("hide_cursor", "/window/hide_cursor", Kind::Bool, "masquer le curseur (écran tactile)"),
    setting("gpu", "/window/gpu", Kind::Bool, "accélération matérielle (+~70 Mo, inutile sur Pi 3)"),
    setting("restart_every_hours", "/supervisor/restart_every_hours", Kind::Number, "redémarrage préventif toutes les N heures (0 = jamais)"),
    setting("max_memory_mb", "/supervisor/max_memory_mb", Kind::Number, "limite RAM du navigateur en Mo (0 = aucune, ex. 350 sur Pi 3)"),
    setting("autostart", "/autostart", Kind::Bool, "lancement au démarrage de la machine"),
    setting("auto_update", "/auto_update", Kind::Bool, "mise à jour automatique"),
    setting("check_interval_secs", "/supervisor/check_interval_secs", Kind::Number, "intervalle de surveillance (s)"),
    setting("failures_before_down", "/supervisor/failures_before_down", Kind::Number, "échecs avant de considérer HA coupé"),
    setting("reload_on_reconnect", "/supervisor/reload_on_reconnect", Kind::Bool, "recharger au retour de HA"),
];

pub fn find(name: &str) -> Result<&'static Setting> {
    SETTINGS.iter().find(|s| s.name == name).with_context(|| format!("réglage inconnu « {name} »\n\n{}", help_text()))
}

/// Liste des réglages pour l'aide de la ligne de commande.
pub fn help_text() -> String {
    let mut text = String::from("Réglages de `ha-kiosk config` :\n");
    for s in SETTINGS {
        let values = match s.kind {
            Kind::Choice(c) => format!(" ({})", c.iter().map(|(v, _)| *v).collect::<Vec<_>>().join(" | ")),
            Kind::Bool => " (oui | non)".into(),
            _ => String::new(),
        };
        text.push_str(&format!("  {:<22} {}{values}\n", s.name, s.help));
    }
    text
}

impl Setting {
    fn get<'a>(&self, cfg: &'a Value) -> Option<&'a Value> {
        cfg.pointer(self.pointer)
    }

    pub fn set(&self, cfg: &mut Value, value: Value) -> Result<()> {
        *cfg.pointer_mut(self.pointer).with_context(|| format!("réglage {} absent", self.name))? = value;
        Ok(())
    }

    /// Valeur lisible, jeton masqué.
    pub fn display(&self, cfg: &Value) -> String {
        match (self.kind, self.get(cfg)) {
            (Kind::Secret, Some(Value::String(t))) if !t.is_empty() => {
                let tail: String = t.chars().skip(t.chars().count().saturating_sub(4)).collect();
                format!("défini (…{tail})")
            }
            (Kind::Secret, _) => "(vide)".into(),
            (Kind::Bool, Some(Value::Bool(b))) => if *b { "oui" } else { "non" }.into(),
            (Kind::List, Some(Value::Array(a))) => a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "),
            (_, Some(Value::String(t))) if t.is_empty() => "(vide)".into(),
            (_, Some(Value::String(t))) => t.clone(),
            (_, Some(other)) => other.to_string(),
            (_, None) => "(non défini)".into(),
        }
    }

    /// Convertit une saisie texte dans le type attendu.
    pub fn parse(&self, raw: &str) -> Result<Value> {
        Ok(match self.kind {
            Kind::Text | Kind::Secret => json!(raw),
            Kind::Bool => json!(prompt::parse_bool(raw).with_context(|| format!("{} attend oui ou non", self.name))?),
            Kind::Number => json!(raw.parse::<u64>().with_context(|| format!("{} attend un nombre", self.name))?),
            Kind::Choice(choices) => {
                if !choices.iter().any(|(v, _)| *v == raw) {
                    let all: Vec<_> = choices.iter().map(|(v, _)| *v).collect();
                    bail!("{} attend : {}", self.name, all.join(" | "));
                }
                raw.parse::<u64>().map(|n| json!(n)).unwrap_or_else(|_| json!(raw))
            }
            Kind::List => json!(raw.split_whitespace().collect::<Vec<_>>()),
        })
    }

    /// Pose la question correspondante, avec la valeur actuelle par défaut.
    pub fn prompt(&self, cfg: &Value) -> Result<Value> {
        let question = format!("{} ({})", self.name, self.help);
        let current = self.get(cfg);
        match self.kind {
            Kind::Secret => Ok(json!(prompt::ask_secret(&question, current.and_then(Value::as_str).unwrap_or(""))?)),
            Kind::Bool => Ok(json!(prompt::ask_bool(&question, current.and_then(Value::as_bool).unwrap_or(false))?)),
            Kind::Number => Ok(json!(prompt::ask_u64(&question, current.and_then(Value::as_u64).unwrap_or(0))?)),
            Kind::Choice(choices) => self.parse(&prompt::choose(&question, choices, &self.display(cfg))?),
            Kind::Text | Kind::List => {
                let shown = self.display(cfg);
                let default = if shown == "(vide)" { "" } else { shown.as_str() };
                self.parse(&prompt::ask(&question, default)?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> Value {
        serde_json::to_value(crate::config::Config::default()).unwrap()
    }

    #[test]
    fn every_setting_points_to_an_existing_field() {
        let cfg = cfg();
        for s in SETTINGS {
            assert!(s.get(&cfg).is_some(), "{} → {}", s.name, s.pointer);
        }
    }

    #[test]
    fn parse_values() {
        assert_eq!(find("rotation").unwrap().parse("180").unwrap(), json!(180));
        assert!(find("rotation").unwrap().parse("45").is_err());
        assert_eq!(find("mode").unwrap().parse("windowed").unwrap(), json!("windowed"));
        assert_eq!(find("gpu").unwrap().parse("oui").unwrap(), json!(true));
        assert!(find("gpu").unwrap().parse("peut-être").is_err());
        assert_eq!(find("command").unwrap().parse("cog  {url}").unwrap(), json!(["cog", "{url}"]));
        assert!(find("inconnu").is_err());
    }

    #[test]
    fn token_is_masked_even_with_non_ascii() {
        let mut cfg = cfg();
        let token = find("token").unwrap();
        token.set(&mut cfg, json!("abcdéfgh")).unwrap();
        assert_eq!(token.display(&cfg), "défini (…éfgh)");
        token.set(&mut cfg, json!("é")).unwrap();
        assert_eq!(token.display(&cfg), "défini (…é)");
    }
}

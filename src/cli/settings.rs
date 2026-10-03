//! Table of settings editable from the command line, shared by
//! `ha-kiosk config` and the `ha-kiosk setup` wizard.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::prompt;
use crate::tr;

/// English and French text.
type Text = (&'static str, &'static str);

#[derive(Clone, Copy)]
pub enum Kind {
    Text,
    Secret,
    Bool,
    Number,
    /// `(value, help)`; numeric values are stored as numbers.
    Choice(&'static [(&'static str, Text)]),
    /// Space-separated arguments.
    List,
}

pub struct Setting {
    pub name: &'static str,
    pointer: &'static str,
    kind: Kind,
    help: Text,
}

const fn setting(name: &'static str, pointer: &'static str, kind: Kind, help: Text) -> Setting {
    Setting { name, pointer, kind, help }
}

pub const SETTINGS: &[Setting] = &[
    setting("language", "/language", Kind::Choice(&[("auto", ("system language", "langue du système")), ("en", ("English", "anglais")), ("fr", ("French", "français"))]), ("interface language", "langue de l'interface")),
    setting("rotation", "/window/rotation", Kind::Choice(&[("0", ("normal", "normale")), ("90", ("portrait", "portrait")), ("180", ("upside down", "retournée")), ("270", ("portrait, flipped", "portrait inversé"))]), ("screen rotation (Linux)", "rotation de l'écran (Linux)")),
    setting("url", "/url", Kind::Text, ("Home Assistant address, e.g. https://192.168.1.10:8123", "adresse de Home Assistant, ex. https://192.168.1.10:8123")),
    setting("dashboard", "/dashboard", Kind::Text, ("dashboard path (empty = default, e.g. lovelace-kiosk/0)", "chemin du dashboard (vide = par défaut, ex. lovelace-kiosk/0)")),
    setting("token", "/token", Kind::Secret, ("long-lived access token (empty = manual login)", "jeton d'accès longue durée (vide = connexion manuelle)")),
    setting("insecure_tls", "/insecure_tls", Kind::Bool, ("accept a self-signed certificate", "accepter un certificat auto-signé")),
    setting("browser", "/browser", Kind::Choice(&[("auto", ("built-in WebView (recommended)", "WebView intégrée (recommandé)")), ("webview", ("built-in WebView", "WebView intégrée")), ("external", ("command: Chromium (token OK), cog, Firefox (no token)", "commande : Chromium (jeton OK), cog, Firefox (sans jeton)"))]), ("browser", "navigateur")),
    setting("command", "/command", Kind::List, ("external browser command ({url} = dashboard)", "commande du navigateur externe ({url} = dashboard)")),
    setting("mode", "/window/mode", Kind::Choice(&[("fullscreen", ("full screen", "plein écran")), ("windowed", ("windowed", "fenêtré")), ("borderless", ("borderless window", "fenêtré sans bordure"))]), ("display mode", "mode d'affichage")),
    setting("width", "/window/width", Kind::Number, ("window width", "largeur de la fenêtre")),
    setting("height", "/window/height", Kind::Number, ("window height", "hauteur de la fenêtre")),
    setting("animations", "/window/animations", Kind::Bool, ("page animations (no = much smoother on a Pi)", "animations des pages (non = bien plus fluide sur Pi)")),
    setting("hide_cursor", "/window/hide_cursor", Kind::Bool, ("hide the cursor (touch screen)", "masquer le curseur (écran tactile)")),
    setting("gpu", "/window/gpu", Kind::Bool, ("hardware acceleration (+~70 MB, useless on a Pi 3)", "accélération matérielle (+~70 Mo, inutile sur Pi 3)")),
    setting("autostart", "/autostart", Kind::Bool, ("start with the machine", "lancement au démarrage de la machine")),
    setting("restart_every_hours", "/supervisor/restart_every_hours", Kind::Number, ("preventive restart every N hours (0 = never)", "redémarrage préventif toutes les N heures (0 = jamais)")),
    setting("max_memory_mb", "/supervisor/max_memory_mb", Kind::Number, ("browser RAM limit in MB (0 = none, e.g. 350 on a Pi 3)", "limite RAM du navigateur en Mo (0 = aucune, ex. 350 sur Pi 3)")),
    setting("auto_update", "/auto_update", Kind::Bool, ("automatic updates", "mise à jour automatique")),
    setting("check_interval_secs", "/supervisor/check_interval_secs", Kind::Number, ("monitoring interval (s)", "intervalle de surveillance (s)")),
    setting("failures_before_down", "/supervisor/failures_before_down", Kind::Number, ("failures before HA is considered down", "échecs avant de considérer HA coupé")),
    setting("reload_on_reconnect", "/supervisor/reload_on_reconnect", Kind::Bool, ("reload when HA is back", "recharger au retour de HA")),
];

fn text((en, fr): Text) -> &'static str {
    tr!(en, fr)
}

pub fn find(name: &str) -> Result<&'static Setting> {
    SETTINGS
        .iter()
        .find(|s| s.name == name)
        .with_context(|| format!("{} « {name} »\n\n{}", tr!("unknown setting", "réglage inconnu"), help_text()))
}

/// Settings list for the command-line help.
pub fn help_text() -> String {
    let mut out = format!("{}\n", tr!("Settings:", "Réglages :"));
    for s in SETTINGS {
        let values = match s.kind {
            Kind::Choice(c) => format!(" ({})", c.iter().map(|(v, _)| *v).collect::<Vec<_>>().join(" | ")),
            Kind::Bool => tr!(" (yes | no)", " (oui | non)").into(),
            _ => String::new(),
        };
        out.push_str(&format!("  {:<22} {}{values}\n", s.name, text(s.help)));
    }
    out
}

impl Setting {
    fn get<'a>(&self, cfg: &'a Value) -> Option<&'a Value> {
        cfg.pointer(self.pointer)
    }

    pub fn set(&self, cfg: &mut Value, value: Value) -> Result<()> {
        *cfg.pointer_mut(self.pointer).with_context(|| format!("setting {} missing", self.name))? = value;
        Ok(())
    }

    /// Readable value, token masked.
    pub fn display(&self, cfg: &Value) -> String {
        let empty = tr!("(empty)", "(vide)");
        match (self.kind, self.get(cfg)) {
            (Kind::Secret, Some(Value::String(t))) if !t.is_empty() => {
                let tail: String = t.chars().skip(t.chars().count().saturating_sub(4)).collect();
                format!("{} (…{tail})", tr!("set", "défini"))
            }
            (Kind::Secret, _) => empty.into(),
            (Kind::Bool, Some(Value::Bool(b))) => if *b { tr!("yes", "oui") } else { tr!("no", "non") }.into(),
            (Kind::List, Some(Value::Array(a))) => a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "),
            (_, Some(Value::String(t))) if t.is_empty() => empty.into(),
            (_, Some(Value::String(t))) => t.clone(),
            (_, Some(other)) => other.to_string(),
            (_, None) => tr!("(not set)", "(non défini)").into(),
        }
    }

    /// Converts typed text to the expected type.
    pub fn parse(&self, raw: &str) -> Result<Value> {
        Ok(match self.kind {
            Kind::Text | Kind::Secret => json!(raw),
            Kind::Bool => json!(prompt::parse_bool(raw).with_context(|| format!("{}: {}", self.name, tr!("yes or no expected", "oui ou non attendu")))?),
            Kind::Number => json!(raw.parse::<u64>().with_context(|| format!("{}: {}", self.name, tr!("a number is expected", "nombre attendu")))?),
            Kind::Choice(choices) => {
                if !choices.iter().any(|(v, _)| *v == raw) {
                    let all: Vec<_> = choices.iter().map(|(v, _)| *v).collect();
                    bail!("{}: {} {}", self.name, tr!("expected", "attendu :"), all.join(" | "));
                }
                raw.parse::<u64>().map(|n| json!(n)).unwrap_or_else(|_| json!(raw))
            }
            Kind::List => json!(raw.split_whitespace().collect::<Vec<_>>()),
        })
    }

    /// Asks the matching question, with the current value as default.
    pub fn prompt(&self, cfg: &Value) -> Result<Value> {
        let question = format!("{} ({})", self.name, text(self.help));
        let current = self.get(cfg);
        match self.kind {
            Kind::Secret => Ok(json!(prompt::ask_secret(&question, current.and_then(Value::as_str).unwrap_or(""))?)),
            Kind::Bool => Ok(json!(prompt::ask_bool(&question, current.and_then(Value::as_bool).unwrap_or(false))?)),
            Kind::Number => Ok(json!(prompt::ask_u64(&question, current.and_then(Value::as_u64).unwrap_or(0))?)),
            Kind::Choice(choices) => {
                let options: Vec<(&str, &str)> = choices.iter().map(|(v, help)| (*v, text(*help))).collect();
                self.parse(&prompt::choose(&question, &options, &self.display(cfg))?)
            }
            Kind::Text | Kind::List => {
                let shown = self.display(cfg);
                let default = if current.is_some_and(|v| v.as_str() == Some("")) { "" } else { shown.as_str() };
                self.parse(&prompt::ask(&question, default)?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{self, Language};

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
        assert_eq!(find("gpu").unwrap().parse("no").unwrap(), json!(false));
        assert!(find("gpu").unwrap().parse("maybe").is_err());
        assert_eq!(find("language").unwrap().parse("fr").unwrap(), json!("fr"));
        assert_eq!(find("command").unwrap().parse("cog  {url}").unwrap(), json!(["cog", "{url}"]));
        assert!(find("unknown").is_err());
    }

    #[test]
    fn token_is_masked_even_with_non_ascii() {
        i18n::set(Language::En);
        let mut cfg = cfg();
        let token = find("token").unwrap();
        token.set(&mut cfg, json!("abcdéfgh")).unwrap();
        assert_eq!(token.display(&cfg), "set (…éfgh)");
        token.set(&mut cfg, json!("é")).unwrap();
        assert_eq!(token.display(&cfg), "set (…é)");
    }
}

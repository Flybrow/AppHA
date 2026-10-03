//! `ha-kiosk config [réglage] [valeur]` : lit ou modifie un seul réglage.

use std::path::Path;

use anyhow::Result;

use super::settings::{self, SETTINGS};
use crate::config::Config;

pub fn run(path: &Path, name: Option<&str>, value: Option<&str>) -> Result<()> {
    let mut cfg = Config::editable_json(path);
    let Some(name) = name else {
        println!("{}", path.display());
        for s in SETTINGS {
            println!("  {:<22} {}", s.name, s.display(&cfg));
        }
        println!("\nModifier : ha-kiosk config <réglage> [valeur]");
        return Ok(());
    };
    let setting = settings::find(name)?;
    let new = match value {
        Some(v) => setting.parse(v)?,
        None => setting.prompt(&cfg)?,
    };
    setting.set(&mut cfg, new)?;
    Config::commit(cfg, path)?;
    println!("{} enregistré dans {}", setting.name, path.display());
    super::apply_hint();
    Ok(())
}

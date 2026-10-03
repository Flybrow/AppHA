//! `ha-kiosk config [name] [value]`: shows or changes one setting.

use std::path::Path;

use anyhow::Result;

use super::settings::{self, SETTINGS};
use crate::config::Config;
use crate::tr;

pub fn run(path: &Path, name: Option<&str>, value: Option<&str>) -> Result<()> {
    let mut cfg = Config::editable_json(path);
    let Some(name) = name else {
        println!("{}", path.display());
        for s in SETTINGS {
            println!("  {:<22} {}", s.name, s.display(&cfg));
        }
        println!("\n{} ha-kiosk config <{}> [{}]", tr!("Change:", "Modifier :"), tr!("name", "nom"), tr!("value", "valeur"));
        return Ok(());
    };
    let setting = settings::find(name)?;
    let new = match value {
        Some(v) => setting.parse(v)?,
        None => setting.prompt(&cfg)?,
    };
    setting.set(&mut cfg, new)?;
    Config::commit(cfg, path)?;
    println!("{} {} {}", setting.name, tr!("saved in", "enregistré dans"), path.display());
    super::apply_hint();
    Ok(())
}

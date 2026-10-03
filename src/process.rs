//! Exécution de commandes système auxiliaires (curl, tar, reg, systemctl).

use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// Exécute sans fenêtre de console (Windows) et renvoie stdout ; erreur avec
/// stderr si la commande échoue.
pub fn run_quiet(cmd: &mut Command) -> Result<Vec<u8>> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.stdin(Stdio::null()).output().with_context(|| format!("lancement de {:?}", cmd.get_program()))?;
    if !out.status.success() {
        bail!("{:?} a échoué : {}", cmd.get_program(), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(out.stdout)
}

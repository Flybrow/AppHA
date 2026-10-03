//! Runs auxiliary system commands (curl, tar, reg, systemctl).

use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

/// Runs without a console window (Windows) and returns stdout; errors with
/// stderr if the command fails.
pub fn run_quiet(cmd: &mut Command) -> Result<Vec<u8>> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let out = cmd.stdin(Stdio::null()).output().with_context(|| format!("starting {:?}", cmd.get_program()))?;
    if !out.status.success() {
        bail!("{:?} failed: {}", cmd.get_program(), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(out.stdout)
}

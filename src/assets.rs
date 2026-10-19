//! The payload files that AutoIt embedded with `FileInstall`.
//!
//! The `.reg` scripts and the SetACL binaries are compiled into the executable
//! and written to `%TEMP%\idm-trial-reset` only when they are needed.

use std::fs;
use std::io;
use std::path::PathBuf;

const RESET_REG: &[u8] = include_bytes!("../assets/idm_reset.reg");
const TRIAL_REG: &[u8] = include_bytes!("../assets/idm_trial.reg");
const REGISTER_REG: &[u8] = include_bytes!("../assets/idm_reg.reg");
const SETACL_X86: &[u8] = include_bytes!("../assets/SetACLx32.exe");
const SETACL_X64: &[u8] = include_bytes!("../assets/SetACLx64.exe");

const REG_FILES: [(&str, &[u8]); 3] = [
    ("idm_reset.reg", RESET_REG),
    ("idm_trial.reg", TRIAL_REG),
    ("idm_reg.reg", REGISTER_REG),
];

/// Directory used instead of AutoIt's bare `@TempDir`.
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("idm-trial-reset")
}

fn write(name: &str, data: &[u8]) -> io::Result<PathBuf> {
    let directory = temp_dir();
    fs::create_dir_all(&directory)?;
    let path = directory.join(name);
    fs::write(&path, data)?;
    Ok(path)
}

/// Extracts one of the three registry scripts and returns its path.
pub fn reg_file(name: &str) -> io::Result<PathBuf> {
    let data = REG_FILES
        .iter()
        .find(|(file, _)| *file == name)
        .map(|(_, data)| *data)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown registry file: {name}"),
            )
        })?;
    write(name, data)
}

/// Extracts the SetACL build that matches the installed Windows architecture
/// (`@OSArch = 'X86' ? SetACLx32.exe : SetACLx64.exe`) and returns its path.
pub fn setacl() -> io::Result<PathBuf> {
    if os_is_64_bit() {
        write("SetACLx64.exe", SETACL_X64)
    } else {
        write("SetACLx32.exe", SETACL_X86)
    }
}

/// Deletes every file this tool extracted (`clearTemp` of the original).
pub fn cleanup() -> io::Result<()> {
    let directory = temp_dir();
    if directory.exists() {
        fs::remove_dir_all(&directory)?;
    }
    Ok(())
}

/// Removes the temporary files when the command finishes.
pub struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = cleanup();
    }
}

/// Whether the operating system is 64-bit, even when called from a 32-bit
/// process (`PROCESSOR_ARCHITEW6432` is set in that case).
fn os_is_64_bit() -> bool {
    if std::env::var_os("PROCESSOR_ARCHITEW6432").is_some() {
        return true;
    }
    match std::env::var("PROCESSOR_ARCHITECTURE") {
        // The common value on 64-bit Windows.
        Ok(architecture) => {
            architecture.eq_ignore_ascii_case("AMD64")
                || architecture.eq_ignore_ascii_case("ARM64")
                || architecture.eq_ignore_ascii_case("IA64")
        }
        Err(_) => cfg!(target_pointer_width = "64"),
    }
}

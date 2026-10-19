//! Registry work: the Rust port of `core.au3`.
//!
//! Every helper here mirrors one AutoIt function (`Reset`, `Trial`,
//! `Register`, `autorun`, `SetOwner`, `SetPermission`, `RegSearch`, ...).

use std::path::Path;

use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_SZ};
use winreg::{RegKey, RegValue};

use crate::assets;
use crate::date;
use crate::search;
use crate::sys;

/// Version compared against the update feed (AutoIt's `$version`).
pub const URL_FORUM: &str = "https://github.com/anhkhoakz/idm-let-me-try-more/discussions";
pub const URL_DOWNLOAD: &str = "https://github.com/anhkhoakz/idm-let-me-try-more/releases";

const DOWNLOAD_MANAGER: &str = r"Software\DownloadManager";
const AUTO_RESET_VALUE: &str = "auto_reset_trial";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "IDM trial reset";

/// The CLSID keys IDM protects. Slot 4 is filled at run time by
/// [`dynamic_key`], keeping the layout of the original `$allkey` array.
const CLSID_KEYS: [&str; 5] = [
    "{6DDF00DB-1234-46EC-8356-27E7B2051192}",
    "{7B8E9164-324D-4A2E-A46D-0165FB2000EC}",
    "{D5B91409-A8CA-4973-9A0B-59F713D25671}",
    "{5ED60779-4DE2-4E07-B862-974CA4FF2E9C}",
    "{07999AC3-058B-40BF-984F-69EB1E554CA7}",
];

/// `HKEY` + `Software\Classes` prefix for HKCU and HKLM.
fn class_hives() -> [(RegKey, &'static str); 2] {
    [
        (RegKey::predef(HKEY_CURRENT_USER), r"Software\Classes"),
        (RegKey::predef(HKEY_LOCAL_MACHINE), r"Software\Classes"),
    ]
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Grants a fresh 30-day trial (`Trial`).
pub fn trial() -> Result<(), String> {
    let keys = class_keys();
    reset(&keys)?;

    println!("  [5/6] importing the trial values");
    import_reg("idm_trial.reg")?;
    println!("  [6/6] locking the keys (read-only, owner: nobody)");
    set_permission(&keys, "read")?;
    set_owner(&keys, "nobody")?;
    Ok(())
}

/// Registers IDM with a fake name and serial (`Register`).
pub fn register(name: &str) -> Result<(), String> {
    let keys = class_keys();
    reset(&keys)?;
    autorun_off()?;

    println!("  [5/7] importing the registration values");
    import_reg("idm_reg.reg")?;

    let dynamic = &keys[4];
    if dynamic.is_empty() {
        println!("  [6/7] no dynamic CLSID key found");
    } else {
        println!("  [6/7] recreating {dynamic}");
        create_class_key(dynamic);
    }

    let manager = download_manager()?;
    manager
        .set_raw_value("FName", &reg_sz(name))
        .map_err(|err| format!(r"cannot write HKCU\{DOWNLOAD_MANAGER}\FName: {err}"))?;

    println!("  [7/7] locking the keys (read-only, owner: nobody)");
    set_permission(&keys, "read")?;
    set_owner(&keys, "nobody")?;
    Ok(())
}

/// Turns the automatic trial reset on (`autorun trial`) or off (`autorun off`).
pub fn autorun_on() -> Result<(), String> {
    let next = date::format(date::add_days(date::today(), 15));
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let (manager, _) = hkcu
        .create_subkey(DOWNLOAD_MANAGER)
        .map_err(|err| format!(r"cannot open HKCU\{DOWNLOAD_MANAGER}: {err}"))?;
    manager
        .set_raw_value(AUTO_RESET_VALUE, &reg_sz(&next))
        .map_err(|err| format!("cannot write {AUTO_RESET_VALUE}: {err}"))?;

    let exe = std::env::current_exe()
        .map_err(|err| format!("cannot locate the current executable: {err}"))?;
    let command = format!("\"{}\" trial --auto", exe.to_string_lossy());

    let (run, _) = hkcu
        .create_subkey(RUN_KEY)
        .map_err(|err| format!("cannot open HKCU\\{RUN_KEY}: {err}"))?;
    run.set_raw_value(RUN_VALUE, &reg_sz(&command))
        .map_err(|err| format!("cannot write the autorun entry: {err}"))?;
    Ok(())
}

/// Removes both autorun values (`autorun off`).
pub fn autorun_off() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(manager) = hkcu.open_subkey(DOWNLOAD_MANAGER) {
        let _ = manager.delete_value(AUTO_RESET_VALUE);
    }
    if let Ok(run) = hkcu.open_subkey(RUN_KEY) {
        let _ = run.delete_value(RUN_VALUE);
    }
    Ok(())
}

/// The date on which the next automatic reset is due, if one is scheduled.
pub fn scheduled_reset_date() -> Option<(i32, u32, u32)> {
    let manager = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(DOWNLOAD_MANAGER)
        .ok()?;
    let value: String = manager.get_value(AUTO_RESET_VALUE).ok()?;
    date::parse(&value)
}

/// The raw content of the `Run` entry (`"<exe>" trial --auto`).
pub fn run_entry() -> Option<String> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .ok()?
        .get_value(RUN_VALUE)
        .ok()
}

/// Name shown by IDM as the registered owner.
pub fn registered_name() -> Option<String> {
    let manager = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(DOWNLOAD_MANAGER)
        .ok()?;
    manager.get_value("FName").ok()
}

/// Whether an automatic reset is scheduled *and* the autorun entry points to
/// an executable that still exists (`isAuto`).
pub fn is_auto() -> bool {
    if scheduled_reset_date().is_none() {
        return false;
    }
    run_entry()
        .and_then(|entry| quoted_path(&entry))
        .is_some_and(|path| std::path::Path::new(&path).exists())
}

// ---------------------------------------------------------------------------
// Reset / permissions
// ---------------------------------------------------------------------------

/// `Reset`: unlock everything, import the reset script, drop the dynamic key.
fn reset(keys: &[String]) -> Result<(), String> {
    println!("  [1/4] taking ownership of the protected keys");
    set_owner(keys, "S-1-1-0")?;
    println!("  [2/4] granting full access");
    set_permission(keys, "full")?;
    println!("  [3/4] importing the reset values");
    import_reg("idm_reset.reg")?;

    let dynamic = &keys[4];
    if dynamic.is_empty() {
        println!("  [4/4] no dynamic CLSID key found");
    } else {
        println!("  [4/4] removing {dynamic}");
        delete_class_key(dynamic);
    }
    Ok(())
}

/// `SetOwner`: `owner` is a SID such as `S-1-1-0` (everyone) or `S-1-0-0`.
fn set_owner(keys: &[String], owner: &str) -> Result<(), String> {
    let setacl = assets::setacl().map_err(|err| format!("cannot extract SetACL: {err}"))?;
    let owner = format!("n:{owner}");

    for key in keys.iter().filter(|key| !key.is_empty()) {
        for target in class_paths(key) {
            let args = [
                "-on",
                target.as_str(),
                "-ot",
                "reg",
                "-actn",
                "setowner",
                "-ownr",
                owner.as_str(),
                "-silent",
            ];
            run_setacl(&setacl, &args)?;
        }
    }
    Ok(())
}

/// `SetPermission`: `permission` is `read` or `full`.
fn set_permission(keys: &[String], permission: &str) -> Result<(), String> {
    let setacl = assets::setacl().map_err(|err| format!("cannot extract SetACL: {err}"))?;
    let ace = format!("n:everyone;p:{permission}");

    for key in keys.iter().filter(|key| !key.is_empty()) {
        for target in class_paths(key) {
            let args = [
                "-on",
                target.as_str(),
                "-ot",
                "reg",
                "-actn",
                "ace",
                "-ace",
                ace.as_str(),
                "-actn",
                "setprot",
                "-op",
                "dacl:p_nc;sacl:p_nc",
                "-silent",
            ];
            run_setacl(&setacl, &args)?;
        }
    }
    Ok(())
}

/// SetACL exits with an error code for keys that do not exist; the original
/// script ignored that too, so only failures to launch are reported.
fn run_setacl(setacl: &Path, args: &[&str]) -> Result<(), String> {
    sys::run(setacl, args).map_err(|err| format!("could not run SetACL: {err}"))?;
    Ok(())
}

/// The four hive paths of one CLSID key, exactly as the original built them.
fn class_paths(key: &str) -> [String; 4] {
    [
        format!(r"HKCU\Software\Classes\CLSID\{key}"),
        format!(r"HKCU\Software\Classes\Wow6432Node\CLSID\{key}"),
        format!(r"HKLM\Software\Classes\CLSID\{key}"),
        format!(r"HKLM\Software\Classes\Wow6432Node\CLSID\{key}"),
    ]
}

/// Every CLSID key that has to be handled, dynamic one included.
fn class_keys() -> Vec<String> {
    let mut keys: Vec<String> = CLSID_KEYS[..4].iter().map(|key| key.to_string()).collect();
    match dynamic_key() {
        Some(dynamic) => {
            println!("  found dynamic CLSID key: {dynamic}");
            keys.push(dynamic);
        }
        None => keys.push(String::new()),
    }
    keys.push(CLSID_KEYS[4].to_string());
    keys
}

fn delete_class_key(guid: &str) {
    for (hive, base) in class_hives() {
        let _ = hive.delete_subkey_all(format!(r"{base}\CLSID\{guid}"));
        let _ = hive.delete_subkey_all(format!(r"{base}\Wow6432Node\CLSID\{guid}"));
    }
}

fn create_class_key(guid: &str) {
    for (hive, base) in class_hives() {
        let _ = hive.create_subkey(format!(r"{base}\CLSID\{guid}"));
        let _ = hive.create_subkey(format!(r"{base}\Wow6432Node\CLSID\{guid}"));
    }
}

/// `RegSearch`: runs `reg query <root> /s` (the old `reg query | findstr`
/// pipeline in one pass) and returns the GUID of the key that holds the needle.
///
/// The AutoIt script ran as a 32-bit process, so its `HKCR\CLSID` lookup saw
/// the WOW6432Node view only; searching both views finds the dynamic key on
/// 64-bit systems as well.
fn dynamic_key() -> Option<String> {
    const ROOTS: [&str; 2] = [r"HKCR\CLSID", r"HKCR\Wow6432Node\CLSID"];
    ROOTS.iter().find_map(|root| search_under(root))
}

fn search_under(root: &str) -> Option<String> {
    let reg = sys::system32("reg.exe");
    let output = sys::capture(reg, &["query", root, "/s"]).ok()?;
    search::guid_in_query_output(&search::decode_output(&output))
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// Writes one of the payload `.reg` files and lets `reg.exe` import it.
fn import_reg(file: &str) -> Result<(), String> {
    let path = assets::reg_file(file).map_err(|err| format!("cannot extract {file}: {err}"))?;
    let path = path.to_string_lossy().into_owned();
    let reg = sys::system32("reg.exe");
    let status =
        sys::run(reg, &["import", &path]).map_err(|err| format!("could not run reg.exe: {err}"))?;
    if !status.success() {
        return Err(format!(
            "`reg import {file}` failed (exit code {:?})",
            status.code()
        ));
    }
    Ok(())
}

fn download_manager() -> Result<RegKey, String> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(DOWNLOAD_MANAGER)
        .map_err(|err| format!(r"cannot open HKCU\{DOWNLOAD_MANAGER}: {err}"))
}

/// `REG_SZ` value with the trailing NUL the registry expects.
fn reg_sz(value: &str) -> RegValue {
    let mut bytes: Vec<u8> = value.encode_utf16().flat_map(u16::to_le_bytes).collect();
    bytes.extend_from_slice(&0u16.to_le_bytes());
    RegValue {
        vtype: REG_SZ,
        bytes,
    }
}

/// Returns the quoted executable of an autorun command line.
fn quoted_path(command: &str) -> Option<String> {
    let start = command.find('"')? + 1;
    let end = command[start..].find('"')? + start;
    if start == end {
        return None;
    }
    Some(command[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GUID: &str = "{12345678-ABCD-1234-ABCD-1234567890AB}";

    #[test]
    fn reads_the_quoted_executable_of_an_autorun_entry() {
        let entry = r#""C:\Tools\idm trial reset.exe" trial --auto"#.to_string();
        assert_eq!(
            quoted_path(&entry),
            Some(r"C:\Tools\idm trial reset.exe".to_string())
        );
        assert_eq!(quoted_path("no quotes at all"), None);
        assert_eq!(quoted_path("\"\" trial"), None);
    }

    #[test]
    fn reg_sz_values_are_utf16_with_a_terminator() {
        let value = reg_sz("AB");
        assert_eq!(value.vtype, REG_SZ);
        assert_eq!(value.bytes, vec![0x41, 0x00, 0x42, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn the_four_class_paths_cover_both_hives_and_views() {
        let paths = class_paths(GUID);
        assert_eq!(
            paths,
            [
                format!(r"HKCU\Software\Classes\CLSID\{GUID}"),
                format!(r"HKCU\Software\Classes\Wow6432Node\CLSID\{GUID}"),
                format!(r"HKLM\Software\Classes\CLSID\{GUID}"),
                format!(r"HKLM\Software\Classes\Wow6432Node\CLSID\{GUID}"),
            ]
        );
    }
}

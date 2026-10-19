//! The sub commands - what the buttons and tabs of the old GUI used to do.

use crate::assets;
use crate::cli::{AutorunArgs, OnOff, RegisterArgs, TrialArgs, UpdateArgs, DEFAULT_NAME};
use crate::core;
use crate::date;
use crate::sys;
use crate::update;

/// `trial`: reset the trial right now (the "Reset the IDM trial now" button).
pub fn trial(args: TrialArgs) -> Result<(), String> {
    if args.auto {
        return trial_when_due(args.no_update_check);
    }

    let _cleanup = assets::CleanupGuard;
    println!("Resetting the IDM trial ...");
    core::trial()?;
    println!("[+] You have a 30-day trial now!");

    if args.autorun {
        core::autorun_on()?;
        println!(
            "[+] The IDM trial will be reset automatically on {}.",
            date::format(date::add_days(date::today(), 15))
        );
    }
    Ok(())
}

/// `trial --auto`: the entry registered in the `Run` key, only acts when the
/// scheduled date has been reached (AutoIt's `TrialSilent`).
fn trial_when_due(skip_update_check: bool) -> Result<(), String> {
    let today = date::today();
    match core::scheduled_reset_date() {
        Some(target) => {
            let days = date::days_between(today, target);
            if days > 0 {
                println!(
                    "Nothing to do: the next automatic reset is {} (in {days} day(s)).",
                    date::format(target)
                );
                return Ok(());
            }
            println!("The scheduled reset date has been reached.");
        }
        None => println!("No automatic reset was scheduled, resetting now."),
    }

    let _cleanup = assets::CleanupGuard;
    println!("Resetting the IDM trial ...");
    core::trial()?;
    core::autorun_on()?;
    println!(
        "[+] You have a 30-day trial now! The next automatic reset is {}.",
        date::format(date::add_days(today, 15))
    );

    if !skip_update_check {
        report_update();
    }
    Ok(())
}

/// `register`: write the fake registration into the registry.
pub fn register(args: RegisterArgs) -> Result<(), String> {
    let trimmed = args.name.trim();
    let name = if trimmed.is_empty() {
        DEFAULT_NAME
    } else {
        trimmed
    };

    let _cleanup = assets::CleanupGuard;
    println!("Registering IDM as \"{name}\" ...");
    core::register(name)?;
    println!("[+] IDM is registered now!");
    Ok(())
}

/// `autorun on` / `autorun off` (the "Automatically" checkbox).
pub fn autorun(args: AutorunArgs) -> Result<(), String> {
    match args.action {
        OnOff::On => {
            core::autorun_on()?;
            println!(
                "[+] The IDM trial will be reset automatically on {}.",
                date::format(date::add_days(date::today(), 15))
            );
        }
        OnOff::Off => {
            core::autorun_off()?;
            println!("[+] The IDM trial will NOT be reset automatically.");
        }
    }
    Ok(())
}

/// `status`: a quick look at what is configured.
pub fn status() -> Result<(), String> {
    println!("IDM trial reset {}", env!("CARGO_PKG_VERSION"));

    match core::scheduled_reset_date() {
        Some(target) => {
            let days = date::days_between(date::today(), target);
            if days > 0 {
                println!("  Automatic reset : enabled (next run in {days} day(s))");
            } else {
                println!("  Automatic reset : enabled (DUE NOW)");
            }
            println!("  Next reset date : {}", date::format(target));
        }
        None => println!("  Automatic reset : disabled"),
    }

    match core::run_entry() {
        Some(entry) => println!("  Autorun entry   : {entry}"),
        None => println!("  Autorun entry   : (not set)"),
    }
    if core::scheduled_reset_date().is_some() && !core::is_auto() {
        println!("  Warning         : the autorun entry is missing or stale - run `autorun on`");
    }
    match core::registered_name() {
        Some(name) if !name.is_empty() => println!("  Registered name : {name}"),
        _ => println!("  Registered name : (not set)"),
    }
    println!("  Forum           : {}", core::URL_FORUM);

    Ok(())
}

/// `update`: ask the feed whether a newer version exists.
pub fn update(args: UpdateArgs) -> Result<(), String> {
    let latest = update::latest_version()
        .ok_or_else(|| "could not check for updates (no network connection?)".to_string())?;

    if latest <= core::VERSION {
        println!(
            "No update was found! (installed: {}, latest: {latest})",
            core::VERSION
        );
        return Ok(());
    }

    println!(
        "[+] Update found: version {latest} (installed: {})",
        core::VERSION
    );
    println!("    {}", core::URL_DOWNLOAD);
    if args.open {
        sys::open(core::URL_DOWNLOAD)?;
    }
    Ok(())
}

/// `clean`: drop the extracted `.reg` files and SetACL binaries.
pub fn clean() -> Result<(), String> {
    let directory = assets::temp_dir();
    if !directory.exists() {
        println!("Nothing to clean.");
        return Ok(());
    }
    assets::cleanup().map_err(|err| format!("cannot remove {}: {err}", directory.display()))?;
    println!("[+] Removed {}.", directory.display());
    Ok(())
}

/// The update prompt of the silent mode, without blocking the scheduled run.
fn report_update() {
    match update::update_available() {
        None => println!("[!] Could not check for updates (no network connection)."),
        Some(false) => {}
        Some(true) => println!("[+] A newer version is available: {}", core::URL_DOWNLOAD),
    }
}

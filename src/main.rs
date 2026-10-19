//! idm-trial-reset - command line rewrite of the original AutoIt tool.
//!
//! Sub commands mirror the tabs of the old GUI:
//!
//! * `trial`    -> "Trial reset" tab (with `--autorun` for the checkbox)
//! * `register` -> "Register" tab
//! * `update`   -> the "Check for update" button
//!
//! Everything that touches HKLM or the protected CLSID keys asks for
//! administrator rights first (the equivalent of AutoIt's `#RequireAdmin`).

mod cli;

#[cfg(windows)]
mod assets;
#[cfg(windows)]
mod commands;
#[cfg(windows)]
mod core;
// Date maths is platform independent so it can be unit tested everywhere; on
// non-Windows only the tests themselves use it.
#[cfg_attr(not(windows), allow(dead_code))]
mod date;
// Same for the `reg query` output parser.
#[cfg_attr(not(windows), allow(dead_code))]
mod search;
// Release tags come from this repository's own GitHub releases; parsing them
// is platform independent too.
#[cfg(windows)]
mod singleton;
#[cfg(windows)]
mod sys;
#[cfg(windows)]
mod update;
#[cfg_attr(not(windows), allow(dead_code))]
mod version;

use clap::Parser;

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let cli = cli::Cli::parse_from(cli::normalized_args(std::env::args()));
    windows_main(cli)
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    // Parse the arguments anyway so the CLI definition is checked on every
    // platform, then explain that the tool itself is Windows only.
    let _ = cli::Cli::parse_from(cli::normalized_args(std::env::args()));
    eprintln!("idm-trial-reset only runs on Windows (it edits the Windows registry).");
    std::process::ExitCode::FAILURE
}

#[cfg(windows)]
fn windows_main(cli: cli::Cli) -> std::process::ExitCode {
    use clap::CommandFactory;

    // Commands that modify the machine ask for elevation before doing
    // anything else (AutoIt's `#RequireAdmin` relaunches the script too).
    let needs_admin = matches!(
        cli.command,
        Some(cli::Command::Trial(_))
            | Some(cli::Command::Register(_))
            | Some(cli::Command::Autorun(_))
    );
    if needs_admin && !sys::is_elevated() {
        return sys::relaunch_elevated();
    }

    // `_Singleton(@ScriptName)` of the original.
    let _instance = match singleton::acquire() {
        Ok(instance) => instance,
        Err(err) => {
            eprintln!("error: {err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    let result = match cli.command {
        None => {
            let mut command = cli::Cli::command();
            let _ = command.print_long_help();
            return std::process::ExitCode::SUCCESS;
        }
        Some(cli::Command::Trial(args)) => commands::trial(args),
        Some(cli::Command::Register(args)) => commands::register(args),
        Some(cli::Command::Autorun(args)) => commands::autorun(args),
        Some(cli::Command::Status) => commands::status(),
        Some(cli::Command::Update(args)) => commands::update(args),
        Some(cli::Command::Clean) => commands::clean(),
    };

    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            std::process::ExitCode::FAILURE
        }
    }
}

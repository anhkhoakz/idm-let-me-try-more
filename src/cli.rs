//! Command line interface definition (clap).

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Name written into IDM's registry when `--name` is not given.
pub const DEFAULT_NAME: &str = "IDM trial reset";

#[derive(Debug, Parser)]
#[command(
    name = "idm-trial-reset",
    version,
    about = "Reset the IDM trial, fix blocked / fake serial errors, or register IDM",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Reset the IDM trial and get a fresh 30-day trial
    Trial(TrialArgs),
    /// Register IDM with a fake name and serial
    Register(RegisterArgs),
    /// Turn the automatic trial reset on or off
    Autorun(AutorunArgs),
    /// Show the current status (autorun, next reset date, registered name, ...)
    Status,
    /// Check whether a newer version of this tool is available
    Update(UpdateArgs),
    /// Delete the temporary files created by this tool
    Clean,
}

#[derive(Debug, Args)]
pub struct TrialArgs {
    /// Only reset when the scheduled reset date has been reached
    /// (this is what the scheduled autorun entry uses)
    #[arg(long = "auto", hide = true)]
    pub auto: bool,

    /// Also schedule an automatic reset 15 days from now
    #[arg(long)]
    pub autorun: bool,

    /// Do not contact the update server afterwards
    #[arg(long, hide = true)]
    pub no_update_check: bool,
}

#[derive(Debug, Args)]
pub struct RegisterArgs {
    /// The name shown as IDM's registered owner
    #[arg(long, default_value = DEFAULT_NAME)]
    pub name: String,
}

#[derive(Debug, Args)]
pub struct AutorunArgs {
    pub action: OnOff,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Open the download page in the browser when an update is available
    #[arg(long)]
    pub open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum OnOff {
    /// Schedule an automatic reset every 15 days
    On,
    /// Remove the automatic reset
    Off,
}

/// Rewrites the legacy switches of the AutoIt version (`/trial`) into the
/// equivalent sub command, so autorun entries created by older releases keep
/// working after the migration to Rust.
pub fn normalized_args<I>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut normalized = vec!["idm-trial-reset".to_string()];
    for arg in args.into_iter().skip(1) {
        match arg.as_str() {
            "/trial" | "--trial" => {
                normalized.push("trial".to_string());
                normalized.push("--auto".to_string());
            }
            other => normalized.push(other.to_string()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn legacy_trial_switch_becomes_a_subcommand() {
        let args = normalized_args(["idm-trial-reset".to_string(), "/trial".to_string()]);
        let cli = Cli::parse_from(&args);
        match cli.command {
            Some(Command::Trial(args)) => {
                assert!(args.auto);
                assert!(!args.autorun);
            }
            other => panic!("expected the trial sub command, got {other:?}"),
        }
    }

    #[test]
    fn double_dash_trial_switch_is_supported_too() {
        let args = normalized_args([
            "idm-trial-reset".into(),
            "--trial".into(),
            "--no-update-check".into(),
        ]);
        let cli = Cli::parse_from(&args);
        match cli.command {
            Some(Command::Trial(args)) => {
                assert!(args.auto);
                assert!(args.no_update_check);
            }
            other => panic!("expected the trial sub command, got {other:?}"),
        }
    }

    #[test]
    fn regular_arguments_are_left_alone() {
        let args = normalized_args([
            "idm-trial-reset".into(),
            "register".into(),
            "--name".into(),
            "Juno".into(),
        ]);
        let cli = Cli::parse_from(&args);
        match cli.command {
            Some(Command::Register(args)) => assert_eq!(args.name, "Juno"),
            other => panic!("expected the register sub command, got {other:?}"),
        }
    }

    #[test]
    fn no_arguments_means_no_subcommand() {
        let cli = Cli::parse_from(normalized_args(["idm-trial-reset".into()]));
        assert!(cli.command.is_none());
    }

    #[test]
    fn autorun_takes_on_and_off() {
        for (value, expected) in [("on", OnOff::On), ("off", OnOff::Off)] {
            let args = normalized_args(["idm-trial-reset".into(), "autorun".into(), value.into()]);
            let cli = Cli::parse_from(&args);
            match cli.command {
                Some(Command::Autorun(args)) => assert_eq!(args.action, expected),
                other => panic!("expected the autorun sub command, got {other:?}"),
            }
        }
    }
}

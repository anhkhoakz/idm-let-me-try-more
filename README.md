<div align="center">

# idm-trial-reset

**Reset the Internet Download Manager trial period from the command line — no cracking, no patched binaries.**

[![Latest release](https://img.shields.io/github/v/release/anhkhoakz/idm-let-me-try-more?include_prereleases&sort=semver)](https://github.com/anhkhoakz/idm-let-me-try-more/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/anhkhoakz/idm-let-me-try-more/total)](https://github.com/anhkhoakz/idm-let-me-try-more/releases)
[![Build status](https://img.shields.io/github/actions/workflow/anhkhoakz/idm-let-me-try-more/ci.yml?branch=main)](https://github.com/anhkhoakz/idm-let-me-try-more/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20only-lightgrey.svg)](#requirements)

Maintained by [anhkhoakz](https://github.com/anhkhoakz) · Based on
[J2TEAM/idm-trial-reset](https://github.com/J2TEAM/idm-trial-reset)

</div>

---

## Disclaimer

> **This project is provided for educational and research purposes only.**
>
> - It is a study of how Windows registry permissions, scheduled tasks and
>   command-line automation work. It is **not** a crack, a keygen, a patch or a
>   pirated copy of any software: it never modifies IDM's binaries and never
>   distributes IDM itself.
> - **Internet Download Manager is commercial software owned by
>   [Tonec FZE](https://www.internetdownloadmanager.com/).** This project is not
>   affiliated with, endorsed by or sponsored by Tonec FZE.
> - **If you find IDM useful, please [buy a genuine
>   license](https://www.internetdownloadmanager.com/buy.html).** A license costs
>   less than most of the problems you will create by not having one.
> - **You are solely responsible for how you use this software.** Using it may
>   breach the terms of service of IDM and, in some jurisdictions, local
>   copyright or contract law. Do **not** use it for any purpose that violates
>   applicable laws or the rights of others. The authors neither encourage nor
>   support such use.
> - The software is provided **"as is"**, without warranty of any kind. No
>   liability is accepted for data loss, licence problems, account bans or any
>   other damage. Use it at your own risk.

By downloading, building or running this project you agree to the terms above
and to the [MIT License](LICENSE).

---

## Table of contents

- [Features](#features)
- [Requirements](#requirements)
- [Download](#download)
- [Verify your download (SHA-256)](#verify-your-download-sha-256)
- [Usage](#usage)
- [Build from source](#build-from-source)
- [Repository layout](#repository-layout)
- [Releases](#releases)
- [Contributing](#contributing)
- [Credits](#credits)
- [License](#license)

## Features

| Feature | Description |
| --- | --- |
| Trial reset | Starts a fresh 30-day trial in a single command |
| Auto reset | Schedules the reset every 15 days (registered startup entry) |
| Registration | Writes a name / serial pair to IDM's registry keys |
| Status | Shows the autorun state, the next reset date and the registered name |
| Fake serial fix | Clears the "fake serial" / blocked-registry state |
| Self-elevating | Requests administrator rights through the UAC prompt when needed |
| Self-updating | `update` checks the release feed for a newer version |
| Portable | Single `.exe`, no installation, no runtime dependencies (CRT is linked statically) |

The registry `.reg` scripts and the [SetACL](https://helgeklein.com/) binaries
in [`assets/`](assets) are embedded into the executable at compile time and
extracted to `%TEMP%\idm-trial-reset` only while a command runs.

## Requirements

- Windows 7 / 8 / 10 / 11, x64 or x86
- Internet Download Manager installed
- Administrator rights (the tool requests elevation itself)

## Download

Grab the latest build from the [Releases](https://github.com/anhkhoakz/idm-let-me-try-more/releases/latest) page.

| Asset | Use it for |
| --- | --- |
| `idm-trial-reset-<version>-windows-x64.zip` | 64-bit Windows (recommended) |
| `idm-trial-reset-<version>-windows-x86.zip` | 32-bit Windows |

Each release also ships a `SHA256SUMS.txt` file. **Always verify the checksum
before running an executable downloaded from the internet** — including ours.

## Verify your download (SHA-256)

Every published archive is hashed at build time and listed in `SHA256SUMS.txt`.

**PowerShell (Windows):**

```powershell
Get-FileHash .\idm-trial-reset-<version>-windows-x64.zip -Algorithm SHA256
Get-Content .\SHA256SUMS.txt
```

**Bash (Linux / macOS / Git Bash):**

```bash
sha256sum -c SHA256SUMS.txt
```

The hash of the file you downloaded must match the one in `SHA256SUMS.txt`.
If it does not, delete the file and download it again.

## Usage

```console
$ idm-trial-reset --help
Usage: idm-trial-reset [COMMAND]

Commands:
  trial     Reset the IDM trial and get a fresh 30-day trial
  register  Register IDM with a fake name and serial
  autorun   Turn the automatic trial reset on or off
  status    Show the current status (autorun, next reset date, registered name, ...)
  update    Check whether a newer version of this tool is available
  clean     Delete the temporary files created by this tool
  help      Print this message or the help of the given subcommand(s)
```

### Examples

```bash
idm-trial-reset trial             # reset the trial now (a fresh 30-day trial)
idm-trial-reset trial --autorun   # ... and schedule an automatic reset in 15 days

idm-trial-reset register --name "Your name"
idm-trial-reset autorun on        # reset the trial automatically every 15 days
idm-trial-reset autorun off
idm-trial-reset status            # show the autorun state and the next reset date
idm-trial-reset update --open     # check for a newer version and open the page
idm-trial-reset clean             # delete the temporary files
```

### Notes

- `trial`, `register` and `autorun` need administrator rights; the tool asks for
  elevation through the UAC prompt by itself.
- `idm-trial-reset --version` prints the running version.
- The legacy switch of the original AutoIt release (`IDM Trial Reset.exe /trial`)
  is still understood, so autorun entries created by previous releases keep
  working.
- Close IDM before running `trial` or `register`.

## Build from source

Install [Rust](https://rustup.rs/), then:

```bash
git clone https://github.com/anhkhoakz/idm-let-me-try-more.git
cd idm-let-me-try-more

cargo build --release      # -> target/release/idm-trial-reset.exe
cargo test                 # unit tests, they run on any platform
cargo fmt --check          # formatting
cargo clippy --all-targets # lints
```

Cross-compile the Windows executable from Linux/macOS:

```bash
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin
cargo xwin build --release --target x86_64-pc-windows-msvc
```

## Repository layout

```
.
├── .github/workflows/   # CI + automated release (SHA-256 checksums)
├── assets/              # .reg scripts + SetACL, embedded into the binary
├── src/                 # Rust CLI source code
├── Cargo.toml
└── LICENSE
```

The crate is fully self-contained: nothing is read from outside this repository
at build time.

## Releases

Releases are fully automated. Push a version tag and GitHub Actions builds,
packages, checksums and publishes it:

```bash
git tag v21.0.0              # must match `version` in Cargo.toml
git push origin v21.0.0
```

The [release workflow](.github/workflows/release.yml):

1. verifies that the tag matches `Cargo.toml`,
2. runs `cargo test` and builds `x86_64` and `i686` release binaries,
3. packages them as `idm-trial-reset-<tag>-windows-{x64,x86}.zip` together with
   `README.md`, `LICENSE` and a `DISCLAIMER.txt`,
4. generates `SHA256SUMS.txt` (SHA-256 of every archive),
5. publishes a GitHub Release with the checksum table in the notes.

## Contributing

Bug reports and pull requests are welcome in
[Discussions](https://github.com/anhkhoakz/idm-let-me-try-more/discussions) and
[Issues](https://github.com/anhkhoakz/idm-let-me-try-more/issues).

Before opening a PR, make sure `cargo fmt --check`, `cargo clippy --all-targets`
and `cargo test` all pass — CI runs them for every change.

Please do not open issues asking for help with piracy, commercial activation or
anything that goes beyond the educational purpose described in the
[Disclaimer](#disclaimer).

## Credits

This is an independent continuation of the work of:

- [idmresettrial](https://github.com/idmresettrial) — original author of the
  registry approach
- [Juno Okyo](https://twitter.com/juno_okyo) / [J2TEAM](https://j2team.org/) —
  the original AutoIt GUI and the first Rust port
  ([J2TEAM/idm-trial-reset](https://github.com/J2TEAM/idm-trial-reset))
- [Helge Klein](https://helgeklein.com/) — [SetACL](https://helgeklein.com/en/setacl/),
  used to restore the registry permissions IDM protects

## License

Distributed under the [MIT License](LICENSE).

```
Copyright (c) 2016-2026 Juno Okyo (J2TEAM), anhkhoakz and contributors
```

See [LICENSE](LICENSE) for the full text, including the educational-use notice.

SetACL is a separate product with its own license; see
<https://helgeklein.com/setacl/>.

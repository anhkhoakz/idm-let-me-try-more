# idm-trial-reset (CLI)

A portable Windows command-line tool that resets the Internet Download Manager
trial period, fixes the "fake serial" state and registers IDM — the Rust port of
the original AutoIt GUI in [`../src`](../src).

> [!WARNING]
> **Educational use only.** This is not a crack, keygen or patch: it never
> modifies IDM's binaries and never distributes IDM. Internet Download Manager
> is commercial software owned by [Tonec FZE](https://www.internetdownloadmanager.com/);
> this project is not affiliated with, endorsed by or sponsored by Tonec FZE.
> **If you find IDM useful, [buy a genuine
> license](https://www.internetdownloadmanager.com/buy.html).** You are solely
> responsible for complying with the laws that apply to you. The software is
> provided "as is", without warranty; use it at your own risk.

Full documentation, download links and SHA-256 verification instructions live in
the [project README](../README.md).

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
```

`trial`, `register` and `autorun` need administrator rights; the tool requests
elevation through the UAC prompt by itself.

## Build

```bash
cargo build --release   # -> target/release/idm-trial-reset.exe
cargo test              # unit tests, they run on any platform
```

Cross-compile from Linux/macOS:

```bash
rustup target add x86_64-pc-windows-msvc
cargo install cargo-xwin
cargo xwin build --release --target x86_64-pc-windows-msvc
```

The `.reg` scripts and the SetACL binaries of [`../src`](../src) are embedded
into the executable at compile time (`include_bytes!`) and extracted to
`%TEMP%\idm-trial-reset` while a command runs.

## License

[MIT](LICENSE) — see the [license file](LICENSE) for the educational-use notice.

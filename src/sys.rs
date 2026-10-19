//! Small Windows helpers: hidden processes, elevation and shell actions.

use std::ffi::OsString;
use std::os::windows::process::CommandExt;
use std::process::{Command, ExitCode, ExitStatus, Stdio};

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_CANCELLED};
use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
use windows_sys::Win32::UI::Shell::{
    IsUserAnAdmin, ShellExecuteExW, ShellExecuteW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOCLOSEPROCESS,
    SHELLEXECUTEINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Keeps child processes out of the taskbar (AutoIt's `@SW_HIDE`).
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// NUL terminated UTF-16 string, as expected by the Win32 API.
pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Full path of a binary in `%SystemRoot%\System32`.
pub fn system32(exe: &str) -> OsString {
    let root = std::env::var_os("SystemRoot").or_else(|| std::env::var_os("windir"));
    match root {
        Some(root) => {
            let mut path = std::path::PathBuf::from(root);
            path.push("System32");
            path.push(exe);
            path.into_os_string()
        }
        None => OsString::from(exe),
    }
}

/// Runs a program with no window and returns its exit status.
pub fn run(program: impl AsRef<std::ffi::OsStr>, args: &[&str]) -> std::io::Result<ExitStatus> {
    Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
}

/// Runs a program hidden and returns what it wrote to stdout.
pub fn capture(program: impl AsRef<std::ffi::OsStr>, args: &[&str]) -> std::io::Result<Vec<u8>> {
    let output = Command::new(program)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .output()?;
    Ok(output.stdout)
}

/// Whether the current process has administrator rights.
pub fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

/// Relaunches this executable through the UAC prompt and exits with the exit
/// code of the elevated instance (AutoIt's `#RequireAdmin` behaviour).
pub fn relaunch_elevated() -> ExitCode {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => {
            eprintln!("error: cannot locate the current executable: {err}");
            return ExitCode::FAILURE;
        }
    };

    let parameters = quote_args(std::env::args().skip(1));
    let verb = wide("runas");
    let file = wide(&exe.to_string_lossy());
    let parameters = wide(&parameters);

    unsafe {
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_FLAG_NO_UI;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = parameters.as_ptr();
        info.nShow = SW_SHOWNORMAL;

        if ShellExecuteExW(&mut info) == 0 {
            let error = GetLastError();
            if error == ERROR_CANCELLED {
                eprintln!(
                    "error: administrator privileges are required (the prompt was cancelled)."
                );
            } else {
                eprintln!("error: failed to relaunch as administrator (Windows error {error}).");
            }
            return ExitCode::FAILURE;
        }

        if info.hProcess.is_null() {
            return ExitCode::SUCCESS;
        }

        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code: u32 = 0;
        GetExitCodeProcess(info.hProcess, &mut code);
        CloseHandle(info.hProcess);
        std::process::exit(code as i32);
    }
}

/// Opens an URL / document with the default handler (`ShellExecuteW "open"`).
pub fn open(url: &str) -> Result<(), String> {
    let verb = wide("open");
    let target = wide(url);
    unsafe {
        let result = ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        // ShellExecuteW returns a value <= 32 as an error code.
        if result as usize <= 32 {
            return Err(format!(
                "could not open {url} (Windows error {})",
                result as usize
            ));
        }
    }
    Ok(())
}

/// Quotes arguments the way the Microsoft C runtime expects them.
pub fn quote_args<I>(args: I) -> String
where
    I: IntoIterator<Item = String>,
{
    args.into_iter()
        .map(quote_arg)
        .collect::<Vec<String>>()
        .join(" ")
}

fn quote_arg(arg: String) -> String {
    if arg.is_empty() {
        return String::from("\"\"");
    }
    if !arg.chars().any(|c| c == ' ' || c == '\t' || c == '"') {
        return arg;
    }

    let mut quoted = String::from("\"");
    let mut backslashes = 0usize;
    for character in arg.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.push_str(&"\\".repeat(backslashes));
                quoted.push(character);
                backslashes = 0;
            }
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_what_needs_quoting() {
        assert_eq!(quote_arg("trial".into()), "trial");
        assert_eq!(quote_arg(String::new()), "\"\"");
        assert_eq!(quote_arg("Juno Okyo".into()), "\"Juno Okyo\"");
        assert_eq!(
            quote_arg(r"C:\dir with space\tool.exe".into()),
            r#""C:\dir with space\tool.exe""#
        );
    }

    #[test]
    fn doubles_trailing_backslashes_and_escapes_quotes() {
        assert_eq!(quote_arg(r#"say "hi""#.into()), r#""say \"hi\"""#);
        assert_eq!(
            quote_arg(r#"C:\path with space\"#.into()),
            r#""C:\path with space\\""#
        );
    }

    #[test]
    fn joins_arguments_into_a_command_line() {
        assert_eq!(
            quote_args([
                "register".to_string(),
                "--name".to_string(),
                "Juno Okyo".to_string()
            ]),
            "register --name \"Juno Okyo\""
        );
    }
}

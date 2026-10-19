//! Update check (AutoIt's `GotUpdate`), implemented with WinHTTP so no TLS
//! crate is needed: the request goes through Windows itself.

use std::ptr;

use windows_sys::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetTimeouts,
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ACCESS_TYPE_NO_PROXY, WINHTTP_FLAG_SECURE,
};

use crate::core::VERSION;
use crate::sys::wide;

/// WinHTTP's `HINTERNET`: an opaque pointer, not a `HANDLE`.
type WinHttpHandle = *mut core::ffi::c_void;

const HOST: &str = "pastebin.com";
const PATH: &str = "/raw/uYr0cstV";
const TIMEOUT_MS: i32 = 15_000;

/// Latest version announced by the feed, or `None` when it cannot be read.
pub fn latest_version() -> Option<i64> {
    let body = http_get(true, HOST, PATH)?;
    let start = body.find("<version>")? + "<version>".len();
    let rest = &body[start..];
    let end = rest.find("</version>")?;
    rest[..end].trim().parse().ok()
}

/// `Some(true)` when a newer version than [`VERSION`] is available.
pub fn update_available() -> Option<bool> {
    latest_version().map(|latest| latest > VERSION)
}

/// RAII wrapper so every early return still closes its handle.
struct InternetHandle(WinHttpHandle);

impl Drop for InternetHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                WinHttpCloseHandle(self.0);
            }
        }
    }
}

/// Plain HTTPS GET, following redirects, returning the body as text.
fn http_get(secure: bool, host: &str, path: &str) -> Option<String> {
    unsafe {
        let agent = wide("idm-trial-reset");
        let mut session = WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        );
        if session.is_null() {
            // Automatic proxy discovery is unavailable on older systems.
            session = WinHttpOpen(
                agent.as_ptr(),
                WINHTTP_ACCESS_TYPE_NO_PROXY,
                ptr::null(),
                ptr::null(),
                0,
            );
        }
        if session.is_null() {
            return None;
        }
        let session = InternetHandle(session);
        WinHttpSetTimeouts(session.0, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS, TIMEOUT_MS);

        let wide_host = wide(host);
        let connection = WinHttpConnect(
            session.0,
            wide_host.as_ptr(),
            if secure { 443 } else { 80 },
            0,
        );
        if connection.is_null() {
            return None;
        }
        let connection = InternetHandle(connection);

        let wide_path = wide(path);
        let verb = wide("GET");
        let request = WinHttpOpenRequest(
            connection.0,
            verb.as_ptr(),
            wide_path.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            if secure { WINHTTP_FLAG_SECURE } else { 0 },
        );
        if request.is_null() {
            return None;
        }
        let request = InternetHandle(request);

        let sent = WinHttpSendRequest(request.0, ptr::null(), 0, ptr::null(), 0, 0, 0);
        if sent == 0 || WinHttpReceiveResponse(request.0, ptr::null_mut()) == 0 {
            return None;
        }

        let mut body: Vec<u8> = Vec::new();
        loop {
            let mut available: u32 = 0;
            if WinHttpQueryDataAvailable(request.0, &mut available) == 0 || available == 0 {
                break;
            }
            let mut buffer = vec![0u8; available as usize];
            let mut read: u32 = 0;
            let ok = WinHttpReadData(request.0, buffer.as_mut_ptr().cast(), available, &mut read);
            if ok == 0 || read == 0 {
                break;
            }
            buffer.truncate(read as usize);
            body.extend_from_slice(&buffer);
        }

        Some(String::from_utf8_lossy(&body).into_owned())
    }
}

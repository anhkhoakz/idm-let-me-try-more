//! Update check against this repository's own GitHub releases, implemented
//! with WinHTTP so no TLS or JSON crate is needed: the request goes through
//! Windows itself.

use std::ptr;

use windows_sys::Win32::Networking::WinHttp::{
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable,
    WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetTimeouts,
    WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ACCESS_TYPE_NO_PROXY, WINHTTP_FLAG_SECURE,
};

use crate::sys::wide;
use crate::version;

/// WinHTTP's `HINTERNET`: an opaque pointer, not a `HANDLE`.
type WinHttpHandle = *mut core::ffi::c_void;

/// `GET /repos/OWNER/REPO/releases/latest` of this repository.
const HOST: &str = "api.github.com";
const PATH: &str = "/repos/anhkhoakz/idm-let-me-try-more/releases/latest";
const TIMEOUT_MS: i32 = 15_000;

/// The newest published release, as reported by the GitHub API.
pub struct LatestRelease {
    /// The tag name, e.g. `v21.0.0`.
    pub tag: String,
    /// The release page to open in a browser, when the API sent one.
    pub url: Option<String>,
}

impl LatestRelease {
    /// `true` when this release is newer than the running executable.
    pub fn is_newer_than_current(&self) -> bool {
        version::is_newer(&self.tag, env!("CARGO_PKG_VERSION"))
    }
}

/// The newest published release, or `None` when it cannot be read (no network,
/// rate limited, no release published yet, ...).
pub fn latest_release() -> Option<LatestRelease> {
    let body = http_get(true, HOST, PATH, &headers())?;
    // `tag_name` is what we compare; `html_url` only matters for `--open`.
    let tag = version::tag_name(&body)?.to_string();
    let url = version::html_url(&body).map(str::to_owned);
    Some(LatestRelease { tag, url })
}

/// `Some(true)` when a newer version than the running one is available.
pub fn update_available() -> Option<bool> {
    latest_release().map(|release| release.is_newer_than_current())
}

/// The headers the GitHub API requires: it rejects requests without a
/// `User-Agent`, and the other two keep the answer predictable.
fn headers() -> String {
    format!(
        "User-Agent: idm-trial-reset/{}\r\n\
         Accept: application/vnd.github+json\r\n\
         X-GitHub-Api-Version: 2022-11-28\r\n",
        env!("CARGO_PKG_VERSION")
    )
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
fn http_get(secure: bool, host: &str, path: &str, headers: &str) -> Option<String> {
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

        // The header block is one wide string, its length excludes the
        // terminating null that `wide` appends.
        let wide_headers = wide(headers);
        let headers_len = wide_headers.len().saturating_sub(1) as u32;
        let sent = WinHttpSendRequest(
            request.0,
            wide_headers.as_ptr(),
            headers_len,
            ptr::null(),
            0,
            0,
            0,
        );
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

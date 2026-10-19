//! Single instance guard, the equivalent of AutoIt's `_Singleton`.

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;

use crate::sys::wide;

/// Keeps the mutex alive for the lifetime of the process.
pub struct Singleton(HANDLE);

impl Drop for Singleton {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Acquires the global mutex, or fails when another instance is running.
pub fn acquire() -> Result<Singleton, String> {
    let name = wide("Local\\idm-trial-reset-singleton");
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        if handle.is_null() {
            return Err(format!(
                "could not create the single-instance mutex (Windows error {})",
                GetLastError()
            ));
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            return Err("another instance of idm-trial-reset is already running".to_string());
        }
        Ok(Singleton(handle))
    }
}

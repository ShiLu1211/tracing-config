//! Windows console support for ANSI color escape codes.
//!
//! On Windows 10+, ANSI escape codes are not enabled by default.
//! This module provides functionality to enable Virtual Terminal Processing.
//!
//! This is called automatically by [`crate::init`] and related functions.
//! You only need to call it manually if you are setting up tracing yourself.
//!
//! # Example
//!
//! ```no_run
//! tracing_declarative::windows::enable_ansi_escapes()
//!     .expect("failed to enable ANSI escapes");
//! ```

/// Enable ANSI virtual terminal processing on Windows 10+.
///
/// On non-Windows platforms this is a no-op that always returns `Ok(())`.
///
/// # Example
///
/// ```no_run
/// tracing_declarative::windows::enable_ansi_escapes()
///     .expect("failed to enable ANSI escapes");
/// ```
#[cfg(windows)]
pub fn enable_ansi_escapes() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5_u32 as u32;
    const ERROR_NOT_ENOUGH_MEMORY: i32 = 8;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(nStdHandle: u32) -> *mut std::ffi::c_void;
        fn GetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, lpMode: *mut u32) -> i32;
        fn SetConsoleMode(hConsoleHandle: *mut std::ffi::c_void, dwMode: u32) -> i32;
    }

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode: u32 = 0;

        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
    Ok(())
}

/// Enable ANSI virtual terminal processing on Windows 10+.
///
/// On non-Windows platforms this is a no-op that always returns `Ok(())`.
///
/// # Example
///
/// ```
/// tracing_declarative::windows::enable_ansi_escapes()
///     .expect("failed to enable ANSI escapes");
/// ```
#[cfg(not(windows))]
pub fn enable_ansi_escapes() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}

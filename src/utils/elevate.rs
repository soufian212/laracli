use std::env;
use std::io;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, CloseHandle};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::Security::{
    GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

/// Check if we're running with elevated privileges
pub fn is_elevated() -> bool {
    unsafe {
        let mut token_handle = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle).is_ok() {
            let mut elevation = TOKEN_ELEVATION::default();
            let mut ret_len = 0u32;
            let res = GetTokenInformation(
                token_handle,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut std::ffi::c_void),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut ret_len,
            );
            let _ = CloseHandle(token_handle);
            if res.is_ok() {
                return elevation.TokenIsElevated != 0;
            }
        }
        false
    }
}

/// Relaunch self with admin privileges (UAC prompt)
pub fn run_as_admin() -> io::Result<()> {
    let exe = env::current_exe()?;
    let exe_str = exe.to_string_lossy();
    
    let args = env::args().skip(1).collect::<Vec<String>>().join(" ");
    
    // Convert strings to wide characters (UTF-16) for Windows API
    let exe_wide: Vec<u16> = exe_str.encode_utf16().chain(Some(0)).collect();
    let verb_wide: Vec<u16> = "runas".encode_utf16().chain(Some(0)).collect();
    let args_wide: Vec<u16> = if args.is_empty() {
        vec![0]
    } else {
        args.encode_utf16().chain(Some(0)).collect()
    };

    unsafe {
        let res = ShellExecuteW(
            None, // HWND - None for no parent window
            PCWSTR::from_raw(verb_wide.as_ptr()),
            PCWSTR::from_raw(exe_wide.as_ptr()),
            if args.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR::from_raw(args_wide.as_ptr())
            },
            PCWSTR::null(), // Working directory (null = current)
            SW_SHOW,
        );

        // ShellExecuteW returns a HINSTANCE, check if > 32 for success
        let result = res.0 as i32;
        if result <= 32 {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("Failed to elevate process. Error code: {}", result)
            ))
        } else {
            // Successfully launched elevated process, exit current one
            std::process::exit(0);
        }
    }
}

/// Example usage: Check elevation and prompt for UAC if needed
pub fn ensure_admin_privileges() -> io::Result<()> {
    if !is_elevated() {
        println!("Administrative privileges required. Requesting elevation...");
        run_as_admin()?;
    }
    println!("Running with administrative privileges.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevation_check() {
        // This will return the actual elevation status
        let elevated = is_elevated();
        println!("Currently elevated: {}", elevated);
    }
}
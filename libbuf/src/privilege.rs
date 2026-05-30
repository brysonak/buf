use anyhow::Result;
use log::{debug, info, warn};

pub fn is_privileged() -> bool {
    #[cfg(unix)]
    {
        use nix::unistd::geteuid;
        let euid = geteuid();
        debug!("EUID = {}", euid);
        return euid.is_root();
    }

    #[cfg(windows)]
    {
        return windows_is_elevated();
    }

    #[cfg(not(any(unix, windows)))]
    {
        warn!("Cannot determine privilege level on this platform, assuming OK");
        return true;
    }
}

pub fn elevate_or_warn(args: &[String]) -> Result<()> {
    warn!("buf must be run as root/Administrator to write to block devices");
    eprintln!("\n  buf requires elevated privileges to write to block devices.\n");

    #[cfg(unix)]
    return unix_elevate(args);

    #[cfg(windows)]
    return windows_elevate(args);

    #[cfg(not(any(unix, windows)))]
    {
        eprintln!("  Please re-run buf with administrator/root privileges.");
        std::process::exit(1);
    }
}

#[cfg(unix)]
fn unix_elevate(args: &[String]) -> Result<()> {
    let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("buf"));

    let sudo = if can_exec("sudo") {
        "sudo"
    } else if can_exec("pkexec") {
        "pkexec"
    } else {
        eprintln!("  Neither sudo nor pkexec found. Please re-run as root.");
        std::process::exit(1);
    };

    eprintln!("  Attempting to re-launch via {}...\n", sudo);
    info!("Re-launching via {} {:?} {:?}", sudo, exe, args);

    let status = std::process::Command::new(sudo)
        .arg(&exe)
        .args(args)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to spawn {}: {}", sudo, e))?;

    std::process::exit(status.code().unwrap_or(1));
}

#[cfg(unix)]
fn can_exec(cmd: &str) -> bool {
    // Check PATH entries directly rather than shelling out to `which`
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = std::path::Path::new(dir).join(cmd);
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

#[cfg(windows)]
fn windows_is_elevated() -> bool {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size,
            &mut size,
        );
        ok.is_ok() && elevation.TokenIsElevated != 0
    }
}

#[cfg(windows)]
fn windows_elevate(args: &[String]) -> Result<()> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

    let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("buf.exe"));
    let cmd_params = format!("/k \"{}\" {}", exe.to_string_lossy(), args.join(" "));

    eprintln!("  Requesting UAC elevation...\n");
    info!("Requesting UAC elevation for {:?} with args {:?}", exe, args);

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    let verb = to_wide("runas");
    let file = to_wide("cmd.exe");
    let param = to_wide(&cmd_params);

    unsafe {
        ShellExecuteW(
            None,
            windows::core::PCWSTR(verb.as_ptr()),
            windows::core::PCWSTR(file.as_ptr()),
            windows::core::PCWSTR(param.as_ptr()),
            None,
            SW_SHOW,
        );
    }

    std::process::exit(0);
}

/*
    buf - Tool for flashing USB drives across platforms
    Copyright (C) 2026 Bryson Kelly

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod flash;
mod style;

fn main() -> eframe::Result {
    // On Windows, trigger a UAC prompt before the window opens so the user
    // only ever sees one window and it's already elevated.
    // On Linux/macOS, elevation via sudo/pkexec breaks display env vars
    // (DISPLAY, WAYLAND_DISPLAY, XAUTHORITY), so we don't attempt it here.
    // The user should just run it with sudo
    #[cfg(target_os = "windows")]
    if !libbuf::is_privileged() {
        windows_elevate();
        // Only reaches here if the user cancelled the UAC prompt.
    }

    libbuf::init_logger(false, false).ok();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("buf")
            .with_inner_size([520.0, 420.0])
            .with_min_inner_size([480.0, 380.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "buf",
        options,
        Box::new(|cc| Ok(Box::new(app::BufApp::new(cc)))),
    )
}

#[cfg(target_os = "windows")]
fn windows_elevate() {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOW;

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0u16)).collect()
    }
    fn path_wide(p: &std::path::Path) -> Vec<u16> {
        p.as_os_str().encode_wide().chain(once(0u16)).collect()
    }

    let verb = to_wide("runas");
    let file = path_wide(&exe);

    let result = unsafe {
        ShellExecuteW(
            None,
            windows::core::PCWSTR(verb.as_ptr()),
            windows::core::PCWSTR(file.as_ptr()),
            None,
            None,
            SW_SHOW,
        )
    };

    // > 32 means the elevated process launched successfully.
    if result.0 as usize > 32 {
        std::process::exit(0);
    }
}

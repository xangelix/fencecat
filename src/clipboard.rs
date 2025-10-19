use std::{
    io::{self, Write as _},
    process::{Command, Stdio},
};

pub fn copy_to_clipboard_multi(text: &str) -> Result<(), String> {
    // 1) Wayland-native CLI (best on Wayland)
    if is_wayland() && cmd_exists("wl-copy") {
        if let Err(e) = wl_copy(text) {
            eprintln!(">> wl-copy failed: {e}");
        } else if verify_wl_paste_non_empty() {
            return Ok(());
        } else {
            eprintln!(">> wl-copy reported success but paste was empty");
        }
    }

    // 2) XWayland/X11 CLI
    if (is_x11() || is_wayland()) && cmd_exists("xclip") {
        if let Err(e) = xclip_copy(text) {
            eprintln!(">> xclip failed: {e}");
        } else if verify_xclip_non_empty() {
            return Ok(());
        }
    }
    if (is_x11() || is_wayland()) && cmd_exists("xsel") {
        if let Err(e) = xsel_copy(text) {
            eprintln!(">> xsel failed: {e}");
        } else if verify_xsel_non_empty() {
            return Ok(());
        }
    }

    // 3) OS-specific fallbacks
    #[cfg(target_os = "macos")]
    {
        if cmd_exists("pbcopy") {
            return pbcopy(text).map_err(|e| e.to_string());
        }
    }
    #[cfg(target_os = "windows")]
    {
        if cmd_exists("clip.exe") {
            return clip_exe(text).map_err(|e| e.to_string());
        }
        if cmd_exists("powershell") {
            return powershell_clip(text).map_err(|e| e.to_string());
        }
    }

    // 4) Library fallback (works on macOS/Windows; may help on some Linux setups)
    if let Err(e) = arboard_fallback(text) {
        return Err(format!("all clipboard backends failed; last error: {e}"));
    }
    Ok(())
}

fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("WAYLAND_SOCKET").is_some()
}
fn is_x11() -> bool {
    std::env::var_os("DISPLAY").is_some()
}

fn cmd_exists(bin: &str) -> bool {
    which::which(bin).is_ok()
}

fn run_with_stdin(bin: &str, args: &[&str], data: &[u8]) -> io::Result<()> {
    let mut child = Command::new(bin)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(data)?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{bin} exited with status {status}"
        )))
    }
}

// wl-clipboard
fn wl_copy(text: &str) -> io::Result<()> {
    // Explicit type avoids some MIME weirdness; -n avoids trailing newline issues.
    run_with_stdin(
        "wl-copy",
        &["--type", "text/plain;charset=utf-8", "-n"],
        text.as_bytes(),
    )
}
fn verify_wl_paste_non_empty() -> bool {
    if !cmd_exists("wl-paste") {
        return true; // can't verify; assume okay
    }
    match Command::new("wl-paste").arg("-n").output() {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => true,
    }
}

// xclip
fn xclip_copy(text: &str) -> io::Result<()> {
    run_with_stdin("xclip", &["-selection", "clipboard"], text.as_bytes())
}
fn verify_xclip_non_empty() -> bool {
    match Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
    {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => true,
    }
}

// xsel
fn xsel_copy(text: &str) -> io::Result<()> {
    run_with_stdin("xsel", &["--clipboard", "--input"], text.as_bytes())
}
fn verify_xsel_non_empty() -> bool {
    match Command::new("xsel")
        .args(["--clipboard", "--output"])
        .output()
    {
        Ok(out) => !out.stdout.is_empty(),
        Err(_) => true,
    }
}

// macOS
#[cfg(target_os = "macos")]
fn pbcopy(text: &str) -> io::Result<()> {
    run_with_stdin("pbcopy", &[], text.as_bytes())
}

#[cfg(target_os = "windows")]
fn clip_exe(text: &str) -> io::Result<()> {
    // clip.exe reads stdin and sets CF_UNICODETEXT
    run_with_stdin("clip.exe", &[], text.as_bytes())
}

// Windows
#[cfg(target_os = "windows")]
fn powershell_clip(text: &str) -> io::Result<()> {
    run_with_stdin(
        "powershell",
        &["-NoProfile", "-Command", "Set-Clipboard"],
        text.as_bytes(),
    )
}

// library fallback (works great on macOS/Windows; mixed on Linux depending on desktop)
fn arboard_fallback(text: &str) -> Result<(), String> {
    match arboard::Clipboard::new() {
        Ok(mut cb) => cb.set_text(text.to_string()).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

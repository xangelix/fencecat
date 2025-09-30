use std::{
    cmp::Reverse,
    collections::HashSet,
    fs::{self, File},
    io::{self, Read, Write as _},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use clap::{ArgAction, Parser};
use ignore::WalkBuilder;

#[derive(Parser, Debug)]
#[command(
    name = "fencecat",
    version,
    about = "Recursively emit Markdown code fences labeled with relative file paths.
Useful for sharing source trees in LLM chats or other issue trackers."
)]
struct Cli {
    /// Root directory to scan
    #[arg(value_name = "DIR", default_value = ".")]
    dir: PathBuf,

    /// Copy the full output to the clipboard
    #[arg(short = 'c', long = "copy", action = ArgAction::SetTrue)]
    copy: bool,

    /// Order files by size (largest first)
    #[arg(short = 'B', long = "biggest-first", action = ArgAction::SetTrue)]
    biggest_first: bool,

    /// Only include files whose extension matches any of the given ones (comma-separated).
    /// Examples: --ext rs,ts,py   or   --ext .md,.toml
    #[arg(
        short,
        long = "ext",
        value_name = "EXT[,EXT...]",
        value_delimiter = ','
    )]
    ext: Option<Vec<String>>,
}

/// Heuristic: consider a file "binary" if the first few KB contain a NUL byte.
/// (Fast and good enough for source trees.)
fn is_binary(path: &Path) -> io::Result<bool> {
    let mut f = File::open(path)?;
    let mut buf = [0u8; 8192];
    let n = f.read(&mut buf)?;
    Ok(buf[..n].contains(&0))
}

fn choose_fence(content: &str) -> String {
    for n in 3..=10 {
        let fence = "`".repeat(n);
        if !content.contains(&fence) {
            return fence;
        }
    }
    "````````````".to_string()
}

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    rel: String,
    size: u64,
}

fn main() {
    let cli = Cli::parse();

    if !cli.dir.is_dir() {
        eprintln!("Not a directory: {}", cli.dir.display());
        std::process::exit(1);
    }

    let ext_filter: Option<HashSet<String>> = cli.ext.as_ref().map(|v| {
        v.iter()
            .map(|s| s.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    });

    let walker = WalkBuilder::new(&cli.dir).build();
    let mut files: Vec<FileInfo> = Vec::new();

    for dent in walker {
        let entry = match dent {
            Ok(e) => e,
            Err(err) => {
                eprintln!("walk error: {err}");
                continue;
            }
        };
        if entry.file_type().is_some_and(|ft| ft.is_file()) {
            let path = entry.path().to_path_buf();

            if let Some(filter) = &ext_filter {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(str::to_ascii_lowercase);
                if !ext.is_some_and(|e| filter.contains(&e)) {
                    continue;
                }
            }

            let md = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("skip {}: metadata error: {e}", path.display());
                    continue;
                }
            };
            if md.len() == 0 {
                continue;
            }

            match is_binary(&path) {
                Ok(true) => continue,
                Ok(false) => {}
                Err(e) => {
                    eprintln!("skip {}: read error: {e}", path.display());
                    continue;
                }
            }

            let rel = rel_string(&cli.dir, &path);

            files.push(FileInfo {
                path,
                rel,
                size: md.len(),
            });
        }
    }

    if cli.biggest_first {
        files.sort_by_key(|f| Reverse(f.size));
    } else {
        files.sort_by(|a, b| a.rel.cmp(&b.rel));
    }

    let mut out = String::new();
    for f in files {
        let bytes = match fs::read(&f.path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("skip {}: read error: {e}", f.path.display());
                continue;
            }
        };
        let content = String::from_utf8_lossy(&bytes);
        let fence = choose_fence(&content);

        out.push_str(&fence);
        out.push_str(&f.rel);
        out.push('\n');

        out.push_str(&content);
        if !content.ends_with('\n') {
            out.push('\n');
        }

        out.push('\n');
        out.push_str(&fence);
        out.push_str("\n\n");
    }

    // Print to stdout
    print!("{out}");

    if cli.copy {
        match copy_to_clipboard_multi(&out) {
            Ok(()) => eprintln!(">> copied to clipboard"),
            Err(e) => eprintln!(">> failed to copy to clipboard: {e}"),
        }
    }
}

fn rel_string(root: &Path, path: &Path) -> String {
    if root == Path::new(".") {
        // Best-effort strip leading "./" from display
        let s = path.to_string_lossy();
        let s = s.strip_prefix("./").unwrap_or(&s);
        s.replace('\\', "/")
    } else {
        path.strip_prefix(root).map_or_else(
            |_| path.to_string_lossy().replace('\\', "/"),
            |p| p.to_string_lossy().replace('\\', "/"),
        )
    }
}

// ---------------- Clipboard helpers ----------------

fn copy_to_clipboard_multi(text: &str) -> Result<(), String> {
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
        // Try PowerShell's clip
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

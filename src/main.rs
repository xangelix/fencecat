use std::{
    cmp::Reverse,
    collections::HashSet,
    fs::{self, File},
    io::{self, Read as _},
    path::{Path, PathBuf},
};

use clap::{ArgAction, Parser};
use ignore::WalkBuilder;

#[allow(clippy::struct_excessive_bools)]
#[derive(Parser, Debug)]
#[command(
    name = "fencecat",
    version,
    about = "Recursively emit Markdown code fences labeled with relative file paths.
Useful for sharing source trees in LLM chats or other issue trackers."
)]
struct Cli {
    /// Root directory to scan OR a single file to emit
    #[arg(value_name = "PATH", default_value = ".")]
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

    /// Include hidden and gitignored files (disable ignore rules)
    #[arg(short = 'H', long = "no-ignore")]
    no_ignore: bool,

    /// Prepend a plain file listing (like `dir`) before the fences (no timestamps/metadata)
    #[arg(short = 'D', long = "dir-list", action = ArgAction::SetTrue)]
    dir_list: bool,
}

impl Cli {
    pub fn build_walkdir(&self) -> WalkBuilder {
        let mut wb = WalkBuilder::new(&self.dir);
        if self.no_ignore {
            wb.hidden(false)
                .ignore(false)
                .git_ignore(false)
                .git_global(false)
                .git_exclude(false)
                .parents(false);
        }
        wb
    }
}

/// Heuristic: consider a file "binary" if the first few KB contain a NUL byte.
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

fn build_ext_filter(cli: &Cli) -> Option<HashSet<String>> {
    cli.ext.as_ref().map(|v| {
        v.iter()
            .map(|s| s.trim().trim_start_matches('.').to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

fn make_fileinfo_if_included(
    path: &Path,
    root_for_rel: &Path,
    ext_filter: Option<&HashSet<String>>,
) -> Option<FileInfo> {
    if let Some(filter) = ext_filter {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !ext.is_some_and(|e| filter.contains(&e)) {
            return None;
        }
    }

    let md = match path.metadata() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("skip {}: metadata error: {e}", path.display());
            return None;
        }
    };
    if md.len() == 0 {
        return None;
    }

    match is_binary(path) {
        Ok(true) => return None,
        Ok(false) => {}
        Err(e) => {
            eprintln!("skip {}: read error: {e}", path.display());
            return None;
        }
    }

    let rel = fencecat::rel_string(root_for_rel, path);

    Some(FileInfo {
        path: path.to_path_buf(),
        rel,
        size: md.len(),
    })
}

fn collect_from_dir(cli: &Cli, ext_filter: Option<&HashSet<String>>) -> Vec<FileInfo> {
    let walker = cli.build_walkdir().build();
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
            let path = entry.path();
            if let Some(info) = make_fileinfo_if_included(path, &cli.dir, ext_filter) {
                files.push(info);
            }
        }
    }

    if cli.biggest_first {
        files.sort_by(|a, b| {
            Reverse(a.size)
                .cmp(&Reverse(b.size))
                .then_with(|| a.rel.cmp(&b.rel))
        });
    } else {
        files.sort_by(|a, b| a.rel.cmp(&b.rel));
    }
    files
}

fn collect_from_single(cli: &Cli, ext_filter: Option<&HashSet<String>>) -> Vec<FileInfo> {
    let path = &cli.dir;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    make_fileinfo_if_included(path, parent, ext_filter)
        .into_iter()
        .collect()
}

fn collect_any(cli: &Cli) -> Vec<FileInfo> {
    let ext_filter = build_ext_filter(cli);

    if !cli.dir.exists() {
        eprintln!("No such file or directory: {}", cli.dir.display());
        std::process::exit(1);
    }

    if cli.dir.is_file() {
        collect_from_single(cli, ext_filter.as_ref())
    } else if cli.dir.is_dir() {
        collect_from_dir(cli, ext_filter.as_ref())
    } else {
        eprintln!("Not a regular file or directory: {}", cli.dir.display());
        std::process::exit(1);
    }
}

fn emit_dir_listing(files: &[FileInfo]) -> String {
    let mut s = String::new();
    s.push_str("```\n");
    for f in files {
        s.push_str(&f.rel);
        s.push('\n');
    }
    s.push_str("```\n\n");
    s
}

fn main() {
    let cli = Cli::parse();

    let files = collect_any(&cli);

    let mut out = String::new();

    if cli.dir_list {
        out.push_str(&emit_dir_listing(&files));
    }

    for f in &files {
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

    print!("{out}");

    if cli.copy {
        match fencecat::clipboard::copy_to_clipboard_multi(&out) {
            Ok(()) => eprintln!(">> copied to clipboard"),
            Err(e) => eprintln!(">> failed to copy to clipboard: {e}"),
        }
    }
}

use std::{
    cmp::Reverse,
    collections::HashSet,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
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

    /// Include hidden and gitignored files (disable ignore rules)
    #[arg(short = 'H', long = "no-ignore")]
    no_ignore: bool,
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

            let rel = fencecat::rel_string(&cli.dir, &path);

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
        match fencecat::clipboard::copy_to_clipboard_multi(&out) {
            Ok(()) => eprintln!(">> copied to clipboard"),
            Err(e) => eprintln!(">> failed to copy to clipboard: {e}"),
        }
    }
}

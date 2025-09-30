# fencecat ⚖️🐈

Recursively emit Markdown code fences labeled with relative file paths.  
Perfect for sharing source trees in LLM chats, issues, blog posts, or code reviews.

## Features

- Walks a directory tree and prints each file inside a fenced code block.
- Labels fences with the file’s relative path.
- Automatically chooses fence length so embedded backticks don’t break.
- Skips binary files and empty files.
- Supports filtering by file extension.
- Optional: order by file size (largest first).
- Optional: copy the entire output to your clipboard.

## Installation


```bash
cargo install --locked fencecat
````

The resulting binary will be in `~/.cargo/bin/fencecat`.

## Usage

```bash
fencecat [OPTIONS] [DIR]
```

### Options

* `-c`, `--copy`
  Copy the full output to the clipboard.
  On Wayland/X11 this uses external tools (`wl-copy`, `xclip`, or `xsel`) if available.

* `-B`, `--biggest-first`
  Order files by size, largest first.

* `--ext EXT[,EXT...]`
  Only include files with the given extensions.
  Examples:

  ```bash
  fencecat . --ext rs,ts,py
  fencecat src --ext .md,.toml
  ```

* `-h`, `--help`
  Show help information.

* `-V`, `--version`
  Show version.

### Examples

Emit all files under `src/`:

```bash
fencecat src
```

Emit only Rust and Python files:

```bash
fencecat . --ext rs,py
```

Copy output to clipboard for pasting into GitHub:

```bash
fencecat . -c
```

Sort files by size:

```bash
fencecat . -B
```

## Clipboard Notes

On Linux/Wayland:

* Install [`wl-clipboard`](https://github.com/bugaevc/wl-clipboard) for `wl-copy` / `wl-paste`.
* If unavailable, `xclip` or `xsel` under XWayland are used.
* macOS uses `pbcopy`; Windows uses PowerShell’s `Set-Clipboard`.

If the clipboard still seems empty, check your compositor or portal logs.

## License

MIT

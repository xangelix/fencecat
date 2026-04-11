# Changelog

## v0.3.0

Support for multiple directory contexts and path positional arguments.

- **Release:** v0.3.0
- **Features (bin):** keep order between multiple contexts
- **Features (bin):** support multiple path positional arguments
- **Chore (meta/Cargo):** structured bin path and name
- **Chore (meta/deps):** bump

## v0.2.2

- **Release:** v0.2.2
- **Chore (docs):** Update README with new cli args

## v0.2.1

- **Release:** v0.2.1
- **Features (bin):** add `regex` and `not-regex` for regex whitelist and blacklist
- **Features (bin):** add extension blacklist/bans
- **Chore (deps):** Bump all dependencies

## v0.2.0

List-dir "table of contents" cli option, deterministic output, and better Windows clipboard fallback prioritization.

- **Release:** v0.2.0
- **Features (bin):** add `dir list` CLI and support passing single files
- **Features (bin):** stable sort output by size, biggest-first
- **Fix (clipboard/Windows):** fall back to `clip.exe` when needed

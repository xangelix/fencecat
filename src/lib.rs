use std::path::Path;

pub mod clipboard;

#[must_use]
pub fn rel_string(root: &Path, path: &Path) -> String {
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

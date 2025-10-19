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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::rel_string;

    #[test]
    fn rel_from_dot_strips_prefix_and_normalizes() {
        let p = Path::new("./src/lib.rs");
        assert_eq!(rel_string(Path::new("."), p), "src/lib.rs");
    }

    #[test]
    fn rel_under_root_normalizes_backslashes() {
        let root = PathBuf::from("repo");
        let mut path = root.clone();
        path.push("src");
        path.push("mod.rs");
        assert_eq!(rel_string(&root, &path), "src/mod.rs");
    }
}

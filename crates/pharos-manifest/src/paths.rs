use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfineError {
    #[error("path must be relative")]
    Absolute,
    #[error("path escapes the repository root")]
    Escapes,
    #[error("path leaves the repository root through a symlink")]
    SymlinkEscape,
}

/// Resolves `rel` against `base` and requires the result to stay inside `root`.
///
/// The check is lexical first (so `..` cannot climb out even when the target does not exist yet),
/// then, if the target exists, on the canonical path (so symlinks cannot point out).
/// `root` must be canonical and `base` inside it.
pub fn confine(root: &Path, base: &Path, rel: &str) -> Result<PathBuf, ConfineError> {
    let mut out = base.to_path_buf();
    for component in Path::new(rel).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
            Component::RootDir | Component::Prefix(_) => return Err(ConfineError::Absolute),
        }
    }
    if !out.starts_with(root) {
        return Err(ConfineError::Escapes);
    }
    match fs::canonicalize(&out) {
        Ok(real) if !real.starts_with(root) => Err(ConfineError::SymlinkEscape),
        Ok(real) => Ok(real),
        Err(_) => Ok(out),
    }
}

/// `path` relative to `root` with `/` separators (`.` for the root itself).
pub fn display_relative(root: &Path, path: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(rel) if rel.as_os_str().is_empty() => ".".into(),
        Ok(rel) => rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// root/
    /// ├─ widget/data/ok.yaml
    /// └─ secret.yaml
    fn setup() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(tmp.path()).unwrap().join("repo");
        fs::create_dir_all(root.join("widget/data")).unwrap();
        fs::write(root.join("widget/data/ok.yaml"), "a: 1").unwrap();
        fs::write(root.join("secret.yaml"), "a: 1").unwrap();
        (tmp, root)
    }

    #[test]
    fn allows_paths_inside_the_repo() {
        let (_tmp, root) = setup();
        let base = root.join("widget");
        assert_eq!(
            confine(&root, &base, "data/ok.yaml"),
            Ok(root.join("widget/data/ok.yaml"))
        );
        // Climbing to a sibling is fine as long as it stays inside the repo root.
        assert_eq!(
            confine(&root, &base, "../secret.yaml"),
            Ok(root.join("secret.yaml"))
        );
        assert_eq!(
            confine(&root, &base, "data/../../secret.yaml"),
            Ok(root.join("secret.yaml"))
        );
        assert_eq!(
            confine(&root, &base, "./missing/"),
            Ok(root.join("widget/missing"))
        );
    }

    #[test]
    fn denies_traversal_and_absolute_paths() {
        let (_tmp, root) = setup();
        let base = root.join("widget");
        assert_eq!(
            confine(&root, &base, "../../etc/passwd"),
            Err(ConfineError::Escapes)
        );
        assert_eq!(
            confine(&root, &base, "data/../../../x"),
            Err(ConfineError::Escapes)
        );
        assert_eq!(
            confine(&root, &base, "/etc/passwd"),
            Err(ConfineError::Absolute)
        );
    }

    #[test]
    fn denies_symlinks_out_of_the_repo() {
        let (tmp, root) = setup();
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("widget/link")).unwrap();
        assert_eq!(
            confine(&root, &root.join("widget"), "link"),
            Err(ConfineError::SymlinkEscape)
        );
    }

    #[test]
    fn displays_relative_paths() {
        let root = Path::new("/r");
        assert_eq!(display_relative(root, Path::new("/r")), ".");
        assert_eq!(display_relative(root, Path::new("/r/a/b.yaml")), "a/b.yaml");
    }
}

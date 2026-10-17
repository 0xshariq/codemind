use std::path::{Component, Path};

/// Default per-file byte ceiling. Reads are bounded even if metadata changes
/// between directory walking and opening the file.
pub const DEFAULT_MAX_FILE_BYTES: u64 = 1_000_000;
/// Maximum number of regular-file and symlink entries considered per snapshot.
pub const DEFAULT_MAX_FILES: usize = 10_000;
/// Hard ceiling on entries (including directories) inspected during a walk.
pub const DEFAULT_MAX_WALK_ENTRIES: usize = 100_000;
/// Maximum total number of captured source bytes in one snapshot.
pub const DEFAULT_MAX_TOTAL_BYTES: u64 = 50_000_000;
/// Maximum portable repository-relative path length accepted by the source layer.
pub const MAX_PATH_BYTES: usize = 4_096;
/// Maximum number of parser diagnostics retained for one source file.
pub const MAX_PARSE_DIAGNOSTICS: usize = 100;

const IGNORED_COMPONENTS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    ".next",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
];

/// Exclude generated/dependency metadata by path component, not substring.
pub fn is_ignored(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .is_some_and(|value| IGNORED_COMPONENTS.contains(&value))
    })
}

/// Convert a repository-relative path into canonical slash-separated form.
///
/// Returns `None` for empty, absolute, drive-prefixed, parent-traversing,
/// non-canonical, control-character, or oversized paths. Paths originating on
/// Windows are normalized using path components; on Unix a literal backslash
/// in a filename is rejected instead of being reinterpreted as a separator.
pub fn normalize_relative_path(path: &Path) -> Option<String> {
    let raw = path.to_str()?;
    if raw.is_empty() || raw.len() > MAX_PATH_BYTES || raw.chars().any(char::is_control) {
        return None;
    }
    if path.is_absolute() {
        return None;
    }
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return None;
    }

    let raw_components: Vec<&str> = raw
        .split(|character| character == '/' || (cfg!(windows) && character == '\\'))
        .collect();
    if raw_components.is_empty()
        || raw_components.iter().any(|part| {
            part.is_empty()
                || *part == "."
                || *part == ".."
                || (!cfg!(windows) && part.contains('\\'))
        })
    {
        return None;
    }

    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                let value = value.to_str()?;
                if value.is_empty()
                    || value == "."
                    || value == ".."
                    || value.contains('/')
                    || value.contains('\\')
                {
                    return None;
                }
                components.push(value);
            }
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return None;
            }
        }
    }
    if components.is_empty() {
        return None;
    }
    Some(components.join("/"))
}

pub fn is_safe_relative(path: &Path) -> bool {
    normalize_relative_path(path).is_some()
}

#[cfg(test)]
mod tests {
    use super::{is_ignored, is_safe_relative, normalize_relative_path};
    use std::path::Path;

    #[test]
    fn validates_portable_repository_relative_paths() {
        assert_eq!(
            normalize_relative_path(Path::new("src/lib.rs")),
            Some("src/lib.rs".into())
        );
        assert!(!is_safe_relative(Path::new("src\\..\\secret.rs")));
        assert!(!is_safe_relative(Path::new("../secret.rs")));
        assert!(!is_safe_relative(Path::new("src/../secret.rs")));
        assert!(!is_safe_relative(Path::new("/etc/passwd")));
        assert!(!is_safe_relative(Path::new("C:\\secret.rs")));
        assert!(!is_safe_relative(Path::new("")));
        assert!(!is_safe_relative(Path::new("src//lib.rs")));
    }

    #[test]
    fn ignores_only_named_path_components() {
        assert!(is_ignored(Path::new("project/node_modules/a.js")));
        assert!(is_ignored(Path::new(".git/config")));
        assert!(!is_ignored(Path::new("src/targeting.rs")));
        assert!(!is_ignored(Path::new("src/build_graph.rs")));
    }
}

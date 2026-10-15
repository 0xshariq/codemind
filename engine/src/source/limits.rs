use std::path::Path;

pub const DEFAULT_MAX_FILE_BYTES: u64 = 1_000_000;

pub fn is_ignored(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".git" | "node_modules" | "target" | ".next" | "dist" | "build")
        )
    })
}

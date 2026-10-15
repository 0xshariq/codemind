use super::{FileEntry, Language, RepositoryContext, SourceManifest, SourceSnapshot, limits};
use ignore::WalkBuilder;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SourceProviderError {
    #[error("source root does not exist: {0}")]
    MissingRoot(PathBuf),
    #[error("source root is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("failed to inspect source: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy)]
pub struct SourceProvider {
    pub max_file_bytes: u64,
}

impl Default for SourceProvider {
    fn default() -> Self {
        Self {
            max_file_bytes: limits::DEFAULT_MAX_FILE_BYTES,
        }
    }
}

impl SourceProvider {
    pub fn resolve(
        &self,
        root: impl AsRef<Path>,
        workspace_id: impl Into<String>,
    ) -> Result<SourceSnapshot, SourceProviderError> {
        let root = root.as_ref().canonicalize()?;
        if !root.exists() {
            return Err(SourceProviderError::MissingRoot(root));
        }
        if !root.is_dir() {
            return Err(SourceProviderError::NotDirectory(root));
        }
        let mut files = Vec::new();
        self.walk(&root, &mut files)?;
        files.sort_by(|left, right| left.path.cmp(&right.path));
        let mut hasher = DefaultHasher::new();
        for file in &files {
            file.path.hash(&mut hasher);
            file.bytes.hash(&mut hasher);
            fs::read(root.join(&file.path))?.hash(&mut hasher);
        }
        let source_snapshot_id = format!("snapshot-{:016x}", hasher.finish());
        let repository_id = format!("repo-{:016x}", stable_hash(&root.to_string_lossy()));
        let context = RepositoryContext {
            repository_id: repository_id.clone(),
            workspace_id: workspace_id.into(),
            source_snapshot_id: source_snapshot_id.clone(),
            graph_version: "day-1".to_owned(),
        };
        Ok(SourceSnapshot {
            root,
            context,
            manifest: SourceManifest {
                repository_id,
                source_snapshot_id,
                files,
            },
        })
    }

    fn walk(&self, root: &Path, files: &mut Vec<FileEntry>) -> Result<(), SourceProviderError> {
        for result in WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(true)
            .build()
        {
            let entry = result.map_err(|error| SourceProviderError::Io(io_error(error)))?;
            let path = entry.path();
            if path == root || !path.is_file() {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .expect("walk path must stay under root");
            if limits::is_ignored(relative) {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|error| SourceProviderError::Io(io_error(error)))?;
            if metadata.len() <= self.max_file_bytes {
                files.push(FileEntry {
                    path: relative.to_string_lossy().replace('\\', "/"),
                    language: Language::from_path(relative),
                    bytes: metadata.len(),
                });
            }

            fn io_error(error: ignore::Error) -> std::io::Error {
                std::io::Error::other(error.to_string())
            }
        }
        Ok(())
    }
}

fn stable_hash(value: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

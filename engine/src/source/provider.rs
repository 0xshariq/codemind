use super::{
    FileEntry, Language, RepositoryContext, SourceManifest, SourceSnapshot, identity, limits,
    manifest::OmittedFile,
};
use ignore::WalkBuilder;
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SourceProviderError {
    #[error("source root does not exist: {0}")]
    MissingRoot(PathBuf),
    #[error("source root is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("failed to inspect or capture source: {0}")]
    Io(#[from] std::io::Error),
    #[error("source exceeds configured processing limits: {0}")]
    LimitExceeded(String),
    #[error("unsafe repository-relative path: {0}")]
    UnsafePath(PathBuf),
    #[error("source provider configuration is invalid: {0}")]
    InvalidConfiguration(String),
    #[error("source identity is invalid: {0}")]
    InvalidRepositoryIdentity(String),
    #[error("source changed during bounded capture: {0}")]
    ChangedDuringCapture(PathBuf),
    #[error("captured source snapshot did not validate: {0}")]
    InvalidSnapshot(String),
}

#[derive(Debug, Clone, Copy)]
pub struct SourceProvider {
    pub max_file_bytes: u64,
    pub max_files: usize,
    pub max_total_bytes: u64,
}

impl Default for SourceProvider {
    fn default() -> Self {
        Self {
            max_file_bytes: limits::DEFAULT_MAX_FILE_BYTES,
            max_files: limits::DEFAULT_MAX_FILES,
            max_total_bytes: limits::DEFAULT_MAX_TOTAL_BYTES,
        }
    }
}

#[derive(Debug)]
struct CandidateFile {
    absolute_path: PathBuf,
    relative_path: String,
    metadata_bytes: u64,
    is_symlink: bool,
}

type CapturedFiles = (
    Vec<FileEntry>,
    std::collections::BTreeMap<String, Vec<u8>>,
    Vec<OmittedFile>,
);

impl SourceProvider {
    /// Capture a local directory using a path-bound repository identity. This
    /// fallback is stable for the same canonical checkout path; callers that
    /// need the same repository ID across different checkouts must call
    /// `resolve_with_repository_identity` with a stable logical repository key.
    pub fn resolve(
        &self,
        root: impl AsRef<Path>,
        workspace_id: impl Into<String>,
    ) -> Result<SourceSnapshot, SourceProviderError> {
        self.resolve_with_repository_identity(root, workspace_id, None)
    }

    /// Capture source under an optional stable logical identity. The optional
    /// identity should come from trusted repository registration (for example,
    /// a CodeMind repository ID or normalized remote URL), not source content.
    pub fn resolve_with_repository_identity(
        &self,
        root: impl AsRef<Path>,
        workspace_id: impl Into<String>,
        repository_identity: Option<&str>,
    ) -> Result<SourceSnapshot, SourceProviderError> {
        self.validate_configuration()?;
        let requested_root = root.as_ref();
        let root_metadata = match fs::metadata(requested_root) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(SourceProviderError::MissingRoot(requested_root.to_owned()));
            }
            Err(error) => return Err(SourceProviderError::Io(error)),
        };
        if !root_metadata.is_dir() {
            return Err(SourceProviderError::NotDirectory(requested_root.to_owned()));
        }
        let canonical_root = fs::canonicalize(requested_root)?;
        if !canonical_root.is_dir() {
            return Err(SourceProviderError::NotDirectory(canonical_root));
        }

        let context_probe = RepositoryContext {
            repository_id: "repo-validation-probe".into(),
            workspace_id: workspace_id.into(),
            source_snapshot_id: "snapshot-validation-probe".into(),
            graph_version: "graph-validation-probe".into(),
        };
        context_probe
            .validate()
            .map_err(|error| SourceProviderError::InvalidConfiguration(error.to_string()))?;
        let workspace_id = context_probe.workspace_id;
        let repository_id = match repository_identity {
            Some(identity_value) => {
                if identity_value.trim().is_empty()
                    || identity_value.len() > limits::MAX_PATH_BYTES
                    || identity_value.chars().any(char::is_control)
                {
                    return Err(SourceProviderError::InvalidRepositoryIdentity(
                        "provided repository identity is empty, too long, or contains control characters"
                            .into(),
                    ));
                }
                identity::repository_id_for_stable_identity(identity_value)
            }
            None => identity::repository_id_for_local_path(&canonical_root),
        };

        let (mut files, captured, mut omitted_files) = self.capture_files(&canonical_root)?;
        files.sort_by(|left, right| left.path.cmp(&right.path));
        omitted_files.sort_by(|left, right| left.path.cmp(&right.path));

        // The manifest identity material is length-prefixed and order-independent.
        // It includes accepted file digests and explicit omission metadata so a
        // partial input cannot masquerade as a complete snapshot of a different set.
        let mut manifest = SourceManifest {
            repository_id: repository_id.clone(),
            source_snapshot_id: String::new(),
            files,
            omitted_files,
        };
        manifest.source_snapshot_id = manifest.recomputed_snapshot_id();
        manifest
            .validate()
            .map_err(|error| SourceProviderError::InvalidSnapshot(error.to_string()))?;

        let context = RepositoryContext {
            repository_id: repository_id.clone(),
            workspace_id,
            source_snapshot_id: manifest.source_snapshot_id.clone(),
            graph_version: identity::graph_version_for_snapshot(&manifest.source_snapshot_id),
        };
        context
            .validate()
            .map_err(|error| SourceProviderError::InvalidConfiguration(error.to_string()))?;

        let snapshot = SourceSnapshot {
            root: canonical_root,
            context,
            manifest,
            files: captured,
        };
        snapshot
            .validate()
            .map_err(|error| SourceProviderError::InvalidSnapshot(error.to_string()))?;
        Ok(snapshot)
    }

    fn validate_configuration(&self) -> Result<(), SourceProviderError> {
        if self.max_file_bytes == 0 {
            return Err(SourceProviderError::InvalidConfiguration(
                "max_file_bytes must be greater than zero".into(),
            ));
        }
        if self.max_files == 0 {
            return Err(SourceProviderError::InvalidConfiguration(
                "max_files must be greater than zero".into(),
            ));
        }
        if self.max_total_bytes == 0 {
            return Err(SourceProviderError::InvalidConfiguration(
                "max_total_bytes must be greater than zero".into(),
            ));
        }
        if self.max_file_bytes > limits::DEFAULT_MAX_TOTAL_BYTES
            || self.max_total_bytes > limits::DEFAULT_MAX_TOTAL_BYTES
            || self.max_files > limits::DEFAULT_MAX_FILES
        {
            return Err(SourceProviderError::InvalidConfiguration(format!(
                "limits cannot exceed {} bytes per/total capture and {} files",
                limits::DEFAULT_MAX_TOTAL_BYTES,
                limits::DEFAULT_MAX_FILES
            )));
        }
        Ok(())
    }

    fn capture_files(&self, root: &Path) -> Result<CapturedFiles, SourceProviderError> {
        let mut candidates = Vec::new();
        let mut visited_entries = 0_usize;
        let root_for_walk = root.to_owned();
        for result in WalkBuilder::new(root)
            .hidden(false)
            .require_git(false)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(true)
            .filter_entry(move |entry| {
                if entry.path() == root_for_walk {
                    return true;
                }
                entry
                    .path()
                    .strip_prefix(&root_for_walk)
                    .map(|relative| !limits::is_ignored(relative))
                    .unwrap_or(false)
            })
            .build()
        {
            let entry = result.map_err(|error| SourceProviderError::Io(io_error(error)))?;
            visited_entries = visited_entries.saturating_add(1);
            if visited_entries > limits::DEFAULT_MAX_WALK_ENTRIES {
                return Err(SourceProviderError::LimitExceeded(format!(
                    "maximum walked entry count {} exceeded",
                    limits::DEFAULT_MAX_WALK_ENTRIES
                )));
            }
            let path = entry.path();
            if path == root {
                continue;
            }

            let metadata = fs::symlink_metadata(path)?;
            let file_type = metadata.file_type();
            let is_symlink = file_type.is_symlink();
            if !is_symlink && !file_type.is_file() {
                continue;
            }

            let relative = path
                .strip_prefix(root)
                .map_err(|_| SourceProviderError::UnsafePath(path.to_owned()))?;
            let relative_path = limits::normalize_relative_path(relative)
                .ok_or_else(|| SourceProviderError::UnsafePath(relative.to_owned()))?;
            if limits::is_ignored(Path::new(&relative_path)) {
                continue;
            }

            if candidates.len() >= self.max_files {
                return Err(SourceProviderError::LimitExceeded(format!(
                    "maximum considered file count {} exceeded",
                    self.max_files
                )));
            }
            candidates.push(CandidateFile {
                absolute_path: path.to_owned(),
                relative_path,
                metadata_bytes: metadata.len(),
                is_symlink,
            });
        }

        // The selected subset is independent of directory-walker enumeration order.
        candidates.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        let mut files = Vec::new();
        let mut captured = std::collections::BTreeMap::new();
        let mut omitted_files = Vec::new();
        let mut total_bytes = 0_u64;

        for candidate in candidates {
            if candidate.is_symlink {
                omitted_files.push(OmittedFile {
                    path: candidate.relative_path,
                    bytes: candidate.metadata_bytes,
                    reason: "symbolic link was not followed for source safety".into(),
                });
                continue;
            }
            if candidate.metadata_bytes > self.max_file_bytes {
                omitted_files.push(OmittedFile {
                    path: candidate.relative_path,
                    bytes: candidate.metadata_bytes,
                    reason: "file exceeds max_file_bytes".into(),
                });
                continue;
            }
            if total_bytes.saturating_add(candidate.metadata_bytes) > self.max_total_bytes {
                omitted_files.push(OmittedFile {
                    path: candidate.relative_path,
                    bytes: candidate.metadata_bytes,
                    reason: "file exceeds remaining total source-byte budget".into(),
                });
                continue;
            }

            // Re-resolve before opening and reject a path that now points outside
            // the captured root. Tree walking never follows links; this is an
            // additional defense against a file being replaced between walk/read.
            let resolved_path = candidate.absolute_path.canonicalize()?;
            if !resolved_path.starts_with(root) {
                return Err(SourceProviderError::UnsafePath(candidate.absolute_path));
            }
            let mut handle = File::open(&resolved_path)?;
            let opened_metadata = handle.metadata()?;
            if !opened_metadata.is_file() {
                return Err(SourceProviderError::ChangedDuringCapture(
                    candidate.absolute_path,
                ));
            }
            let opened_len = opened_metadata.len();
            if opened_len > self.max_file_bytes {
                omitted_files.push(OmittedFile {
                    path: candidate.relative_path,
                    bytes: opened_len,
                    reason: "file grew beyond max_file_bytes before capture".into(),
                });
                continue;
            }
            if total_bytes.saturating_add(opened_len) > self.max_total_bytes {
                omitted_files.push(OmittedFile {
                    path: candidate.relative_path,
                    bytes: opened_len,
                    reason: "file exceeds remaining total source-byte budget".into(),
                });
                continue;
            }

            let mut content = Vec::with_capacity(opened_len.min(self.max_file_bytes) as usize);
            handle
                .by_ref()
                .take(self.max_file_bytes.saturating_add(1))
                .read_to_end(&mut content)?;
            let after_len = handle.metadata()?.len();
            if content.len() as u64 != opened_len || after_len != opened_len {
                return Err(SourceProviderError::ChangedDuringCapture(
                    candidate.absolute_path,
                ));
            }
            let content_bytes = content.len() as u64;
            if content_bytes > self.max_file_bytes {
                omitted_files.push(OmittedFile {
                    path: candidate.relative_path,
                    bytes: content_bytes,
                    reason: "file exceeded max_file_bytes during bounded read".into(),
                });
                continue;
            }
            if total_bytes.saturating_add(content_bytes) > self.max_total_bytes {
                omitted_files.push(OmittedFile {
                    path: candidate.relative_path,
                    bytes: content_bytes,
                    reason: "file exceeds remaining total source-byte budget".into(),
                });
                continue;
            }

            total_bytes = total_bytes.saturating_add(content_bytes);
            let relative_path = candidate.relative_path;
            let language = Language::from_path(Path::new(&relative_path));
            let digest = identity::digest_bytes(&content);
            files.push(FileEntry {
                path: relative_path.clone(),
                language,
                bytes: content_bytes,
                digest,
            });
            captured.insert(relative_path, content);
        }

        Ok((files, captured, omitted_files))
    }
}

fn io_error(error: ignore::Error) -> std::io::Error {
    std::io::Error::other(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{SourceProvider, SourceProviderError};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn captures_pinned_source_and_records_oversized_files() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.path().join("large.bin"), vec![b'x'; 128]).unwrap();
        let provider = SourceProvider {
            max_file_bytes: 32,
            max_files: 20,
            max_total_bytes: 256,
        };
        let snapshot = provider.resolve(root.path(), "workspace-a").unwrap();
        assert_eq!(snapshot.file_text("main.rs").unwrap(), "fn main() {}\n");
        assert_eq!(snapshot.manifest.omitted_files.len(), 1);
        assert_eq!(snapshot.manifest.omitted_files[0].path, "large.bin");
        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn prunes_builtin_ignored_directories_before_counting_walk_entries() {
        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("node_modules")).unwrap();
        fs::create_dir(root.path().join("node_modules/pkg")).unwrap();
        fs::write(root.path().join("node_modules/pkg/generated.js"), "ignored").unwrap();
        fs::write(root.path().join("main.rs"), "fn main() {}\n").unwrap();

        let provider = SourceProvider {
            max_file_bytes: 64,
            max_files: 10,
            max_total_bytes: 256,
        };
        let snapshot = provider.resolve(root.path(), "workspace-a").unwrap();

        assert!(snapshot.files.contains_key("main.rs"));
        assert!(
            !snapshot
                .files
                .keys()
                .any(|path| path.starts_with("node_modules/"))
        );
        assert!(
            !snapshot
                .manifest
                .omitted_files
                .iter()
                .any(|entry| entry.path.starts_with("node_modules/"))
        );
    }

    #[test]
    fn total_byte_budget_omits_files_without_publishing_them_as_captured() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("a.py"), "def a(): pass\n").unwrap();
        fs::write(root.path().join("b.py"), "def b(): pass\n").unwrap();
        let provider = SourceProvider {
            max_file_bytes: 64,
            max_files: 10,
            max_total_bytes: 16,
        };
        let snapshot = provider.resolve(root.path(), "workspace-a").unwrap();
        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.manifest.omitted_files.len(), 1);
        assert!(
            snapshot.manifest.omitted_files[0]
                .reason
                .contains("remaining total source-byte budget")
        );
        assert!(
            snapshot
                .files
                .values()
                .map(|bytes| bytes.len() as u64)
                .sum::<u64>()
                <= 16
        );
        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn stable_repository_identity_survives_a_different_checkout_path() {
        let first_root = tempdir().unwrap();
        let second_root = tempdir().unwrap();
        for root in [first_root.path(), second_root.path()] {
            fs::write(root.join("lib.py"), "def f():\n    return 1\n").unwrap();
        }
        let provider = SourceProvider::default();
        let first = provider
            .resolve_with_repository_identity(first_root.path(), "workspace", Some("repo-key"))
            .unwrap();
        let second = provider
            .resolve_with_repository_identity(second_root.path(), "workspace", Some("repo-key"))
            .unwrap();
        assert_eq!(first.context.repository_id, second.context.repository_id);
        assert_eq!(
            first.context.source_snapshot_id,
            second.context.source_snapshot_id
        );
        assert_eq!(first.context.graph_version, second.context.graph_version);
    }

    #[test]
    fn invalid_limits_and_non_directory_roots_are_explicit_errors() {
        let root = tempdir().unwrap();
        let invalid = SourceProvider {
            max_file_bytes: 0,
            ..SourceProvider::default()
        };
        assert!(matches!(
            invalid.resolve(root.path(), "workspace"),
            Err(SourceProviderError::InvalidConfiguration(_))
        ));
        let file = root.path().join("file.txt");
        fs::write(&file, "text").unwrap();
        assert!(matches!(
            SourceProvider::default().resolve(&file, "workspace"),
            Err(SourceProviderError::NotDirectory(_))
        ));
    }
}

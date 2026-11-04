//! Canonical filesystem paths for CodeMind's local state.
//!
//! The root is always `<home>/.codemind`; it is never inferred from the current
//! working directory or from the repository being analysed. Path-builder
//! methods validate every caller-controlled path component before joining it.

use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io,
    path::{Path, PathBuf},
};

const CODEMIND_DIR_NAME: &str = ".codemind";
const REPOSITORIES_DIR_NAME: &str = "repositories";
const WORKSPACES_DIR_NAME: &str = "workspaces";
const CONFIG_DIR_NAME: &str = "config";
const LOCKS_DIR_NAME: &str = "locks";
const REPOSITORY_METADATA_FILE: &str = "repository.json";
const REPOSITORY_SCOPES_DIR_NAME: &str = "scopes";
const REPOSITORY_STATE_FILE: &str = "state.json";
const WORKSPACE_MARKER_FILE: &str = "workspace.json";
const WORKSPACE_REPOSITORY_DIR_NAME: &str = "repository";

/// Errors raised while resolving or preparing CodeMind's local directories.
#[derive(Debug, thiserror::Error)]
pub enum CodeMindPathsError {
    /// The operating system did not provide a usable home directory.
    #[error("could not resolve the user's home directory")]
    HomeDirectoryUnavailable,

    /// A supplied home path must be absolute so the root cannot depend on the
    /// process's current working directory.
    #[error("home directory path must be absolute: `{path:?}`")]
    HomeDirectoryNotAbsolute { path: PathBuf },

    /// A caller-controlled identifier is not safe to use as a single path
    /// component.
    #[error("invalid {kind} path component `{value}`; use only ASCII letters, digits, '-' or '_'")]
    InvalidPathComponent { kind: &'static str, value: String },

    /// A path expected to be a directory is a regular file or another type.
    #[error("expected a directory at `{path:?}`")]
    NotDirectory { path: PathBuf },

    /// A CodeMind directory must not itself be a symbolic link.
    #[error("refusing to use a symbolic link as a CodeMind directory: `{path:?}`")]
    SymlinkDirectory { path: PathBuf },

    /// A metadata record must be a regular file, not a symbolic link.
    #[error("refusing to read a symbolic link as CodeMind metadata: `{path:?}`")]
    SymlinkFile { path: PathBuf },

    /// A metadata path must resolve to a regular file.
    #[error("expected a regular metadata file at `{path:?}`")]
    NotRegularFile { path: PathBuf },

    /// Creating or inspecting a directory failed.
    #[error("filesystem operation failed for `{path:?}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Restricting directory permissions failed on a Unix-like platform.
    #[error("could not restrict permissions for CodeMind directory `{path:?}`: {source}")]
    Permissions {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Acquiring a repository-scope operating-system lock failed.
    #[error("could not acquire repository-scope lock `{path:?}`: {source}")]
    LockIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

/// A cross-process exclusive lock for one `(repository_id, workspace_scope_key)`.
///
/// The OS releases the lock when the file handle is closed, including when the
/// process exits unexpectedly. The lock file itself is intentionally retained:
/// unlinking a locked file could let another process create and lock a different
/// inode for the same logical scope.
#[derive(Debug)]
pub struct RepositoryScopeLock {
    _file: File,
}

/// Canonical paths for CodeMind's durable metadata and temporary workspaces.
///
/// Construct this with [`CodeMindPaths::discover`] in production. Tests should
/// use [`CodeMindPaths::from_home_dir`] with a temporary directory rather than
/// modifying process-global home-directory environment variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeMindPaths {
    root: PathBuf,
}

impl CodeMindPaths {
    /// Resolve the current user's home directory using the `dirs` crate.
    ///
    /// No working-directory fallback is used. If home resolution fails, the
    /// caller gets an explicit error and may decide whether startup can continue.
    pub fn discover() -> Result<Self, CodeMindPathsError> {
        let home = dirs::home_dir().ok_or(CodeMindPathsError::HomeDirectoryUnavailable)?;
        Self::from_home_dir(home)
    }

    /// Construct paths from an explicit home directory.
    ///
    /// This constructor is especially useful for isolated tests. The supplied
    /// path must be absolute; the directory does not need to exist yet.
    pub fn from_home_dir(home: impl AsRef<Path>) -> Result<Self, CodeMindPathsError> {
        let home = home.as_ref();
        if !home.is_absolute() {
            return Err(CodeMindPathsError::HomeDirectoryNotAbsolute {
                path: home.to_path_buf(),
            });
        }

        Ok(Self {
            root: home.join(CODEMIND_DIR_NAME),
        })
    }

    /// The canonical CodeMind root: `<home>/.codemind`.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Durable repository metadata root.
    pub fn repositories_dir(&self) -> PathBuf {
        self.root.join(REPOSITORIES_DIR_NAME)
    }

    /// Temporary, operation-owned workspace root.
    pub fn workspaces_dir(&self) -> PathBuf {
        self.root.join(WORKSPACES_DIR_NAME)
    }

    /// Validate the canonical CodeMind/workspaces directory chain without
    /// creating it. `None` means startup has no workspace root yet; symlinked
    /// roots and non-directory components are rejected.
    pub(crate) fn existing_workspaces_dir(&self) -> Result<Option<PathBuf>, CodeMindPathsError> {
        match fs::symlink_metadata(&self.root) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(CodeMindPathsError::Io {
                    path: self.root.clone(),
                    source,
                });
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(CodeMindPathsError::SymlinkDirectory {
                    path: self.root.clone(),
                });
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(CodeMindPathsError::NotDirectory {
                    path: self.root.clone(),
                });
            }
            Ok(_) => {}
        }
        ensure_existing_private_directory(&self.root)?;

        let workspaces = self.workspaces_dir();
        match fs::symlink_metadata(&workspaces) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(CodeMindPathsError::Io {
                    path: workspaces,
                    source,
                });
            }
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(CodeMindPathsError::SymlinkDirectory { path: workspaces });
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(CodeMindPathsError::NotDirectory { path: workspaces });
            }
            Ok(_) => {}
        }
        ensure_existing_private_directory(&workspaces)?;
        Ok(Some(workspaces))
    }

    /// Cross-process coordination lock directory.
    pub fn locks_dir(&self) -> PathBuf {
        self.root.join(LOCKS_DIR_NAME)
    }

    /// Acquire an exclusive OS-backed lock for a repository/workspace scope.
    ///
    /// The lock is safe across multiple engine processes sharing this
    /// `~/.codemind` root. Blocking acquisition must be performed from a
    /// blocking thread when called by an async request handler.
    pub fn acquire_repository_scope_lock(
        &self,
        repository_id: &str,
        workspace_scope_key: &str,
    ) -> Result<RepositoryScopeLock, CodeMindPathsError> {
        let file = self.open_scope_lock_file(repository_id, workspace_scope_key)?;
        let path = self.scope_lock_path(repository_id, workspace_scope_key)?;
        file.lock()
            .map_err(|source| CodeMindPathsError::LockIo { path, source })?;
        Ok(RepositoryScopeLock { _file: file })
    }

    /// Try to acquire an exclusive lock without blocking. `None` means another
    /// process currently owns the scope and recovery must leave it untouched.
    pub fn try_acquire_repository_scope_lock(
        &self,
        repository_id: &str,
        workspace_scope_key: &str,
    ) -> Result<Option<RepositoryScopeLock>, CodeMindPathsError> {
        let file = self.open_scope_lock_file(repository_id, workspace_scope_key)?;
        let path = self.scope_lock_path(repository_id, workspace_scope_key)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(RepositoryScopeLock { _file: file })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(source)) => Err(CodeMindPathsError::LockIo { path, source }),
        }
    }

    fn open_scope_lock_file(
        &self,
        repository_id: &str,
        workspace_scope_key: &str,
    ) -> Result<File, CodeMindPathsError> {
        validate_component("repository ID", repository_id)?;
        validate_component("workspace key", workspace_scope_key)?;
        self.ensure_base_dirs()?;
        let path = self.scope_lock_path(repository_id, workspace_scope_key)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(CodeMindPathsError::SymlinkFile { path });
            }
            Ok(metadata) if !metadata.is_file() => {
                return Err(CodeMindPathsError::NotRegularFile { path });
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(source) => return Err(CodeMindPathsError::Io { path, source }),
        }

        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options
            .open(&path)
            .map_err(|source| CodeMindPathsError::LockIo {
                path: path.clone(),
                source,
            })?;
        let metadata = file
            .metadata()
            .map_err(|source| CodeMindPathsError::LockIo {
                path: path.clone(),
                source,
            })?;
        if !metadata.is_file() {
            return Err(CodeMindPathsError::NotRegularFile { path });
        }
        // Re-check after open so a pre-existing symlink cannot be accepted.
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(CodeMindPathsError::SymlinkFile { path });
            }
            Ok(_) => {}
            Err(source) => return Err(CodeMindPathsError::Io { path, source }),
        }
        Ok(file)
    }

    fn scope_lock_path(
        &self,
        repository_id: &str,
        workspace_scope_key: &str,
    ) -> Result<PathBuf, CodeMindPathsError> {
        validate_component("repository ID", repository_id)?;
        validate_component("workspace key", workspace_scope_key)?;
        let mut digest = Sha256::new();
        digest.update(repository_id.as_bytes());
        digest.update([0]);
        digest.update(workspace_scope_key.as_bytes());
        let digest = digest.finalize();
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut encoded_digest = String::with_capacity(digest.len() * 2);
        for byte in digest.iter().copied() {
            encoded_digest.push(HEX[(byte >> 4) as usize] as char);
            encoded_digest.push(HEX[(byte & 0x0f) as usize] as char);
        }

        let name = format!("repository-{encoded_digest}.lock");
        Ok(self.locks_dir().join(name))
    }

    /// Reserved path for future file-based configuration.
    ///
    /// This method only returns the path. The directory is intentionally not
    /// created by [`CodeMindPaths::ensure_base_dirs`] while configuration remains
    /// environment-driven.
    pub fn config_dir(&self) -> PathBuf {
        self.root.join(CONFIG_DIR_NAME)
    }

    /// Path for one repository's durable metadata directory.
    pub fn repository_dir(&self, repository_id: &str) -> Result<PathBuf, CodeMindPathsError> {
        let repository_id = validate_component("repository ID", repository_id)?;
        Ok(self.repositories_dir().join(repository_id))
    }

    /// Path to a repository's stable metadata record.
    pub fn repository_metadata_path(
        &self,
        repository_id: &str,
    ) -> Result<PathBuf, CodeMindPathsError> {
        Ok(self
            .repository_dir(repository_id)?
            .join(REPOSITORY_METADATA_FILE))
    }

    /// Validate the existing directory chain and regular-file entry for a
    /// repository metadata record before it is read. This method never creates
    /// missing paths and never follows a symlinked directory or metadata file.
    pub(crate) fn existing_repository_metadata_path(
        &self,
        repository_id: &str,
    ) -> Result<PathBuf, CodeMindPathsError> {
        let repository_dir = self.repository_dir(repository_id)?;
        ensure_existing_private_directory(&self.root)?;
        ensure_existing_private_directory(&self.repositories_dir())?;
        ensure_existing_private_directory(&repository_dir)?;

        let path = self.repository_metadata_path(repository_id)?;
        ensure_regular_metadata_file(&path)?;
        Ok(path)
    }

    /// Validate the existing directory chain and regular-file entry for a
    /// workspace-scoped state record before it is read.
    pub(crate) fn existing_repository_state_path(
        &self,
        repository_id: &str,
        workspace_key: &str,
    ) -> Result<PathBuf, CodeMindPathsError> {
        let repository_dir = self.repository_dir(repository_id)?;
        let scopes_dir = repository_dir.join(REPOSITORY_SCOPES_DIR_NAME);
        let scope_dir = self.repository_scope_dir(repository_id, workspace_key)?;
        ensure_existing_private_directory(&self.root)?;
        ensure_existing_private_directory(&self.repositories_dir())?;
        ensure_existing_private_directory(&repository_dir)?;
        ensure_existing_private_directory(&scopes_dir)?;
        ensure_existing_private_directory(&scope_dir)?;

        let path = self.repository_state_path(repository_id, workspace_key)?;
        ensure_regular_metadata_file(&path)?;
        Ok(path)
    }

    /// Path for one repository's workspace/security-scope metadata directory.
    pub fn repository_scope_dir(
        &self,
        repository_id: &str,
        workspace_key: &str,
    ) -> Result<PathBuf, CodeMindPathsError> {
        let repository_id = validate_component("repository ID", repository_id)?;
        let workspace_key = validate_component("workspace key", workspace_key)?;
        Ok(self
            .repositories_dir()
            .join(repository_id)
            .join(REPOSITORY_SCOPES_DIR_NAME)
            .join(workspace_key))
    }

    /// Path to the latest verified repository state for a workspace scope.
    pub fn repository_state_path(
        &self,
        repository_id: &str,
        workspace_key: &str,
    ) -> Result<PathBuf, CodeMindPathsError> {
        Ok(self
            .repository_scope_dir(repository_id, workspace_key)?
            .join(REPOSITORY_STATE_FILE))
    }

    /// Path for one operation's temporary workspace directory.
    pub fn workspace_dir(&self, operation_id: &str) -> Result<PathBuf, CodeMindPathsError> {
        let operation_id = validate_component("operation ID", operation_id)?;
        Ok(self.workspaces_dir().join(operation_id))
    }

    /// Path to the operation workspace's ownership/lifecycle marker.
    pub fn workspace_marker_path(&self, operation_id: &str) -> Result<PathBuf, CodeMindPathsError> {
        Ok(self
            .workspace_dir(operation_id)?
            .join(WORKSPACE_MARKER_FILE))
    }

    /// Destination where the temporary Git checkout will be placed.
    pub fn workspace_repository_dir(
        &self,
        operation_id: &str,
    ) -> Result<PathBuf, CodeMindPathsError> {
        Ok(self
            .workspace_dir(operation_id)?
            .join(WORKSPACE_REPOSITORY_DIR_NAME))
    }

    /// Create the durable root, `repositories/`, and `workspaces/` directories.
    ///
    /// On Unix, these directories are restricted to the current user (0700).
    /// `config/` is intentionally left uncreated until a file-based config is
    /// actually introduced. A directory that is a symbolic link is rejected.
    pub fn ensure_base_dirs(&self) -> Result<(), CodeMindPathsError> {
        ensure_private_directory(&self.root)?;
        ensure_private_directory(&self.repositories_dir())?;
        ensure_private_directory(&self.workspaces_dir())?;
        ensure_private_directory(&self.locks_dir())?;
        Ok(())
    }

    /// Create and return a repository's durable metadata directory.
    pub fn ensure_repository_dir(
        &self,
        repository_id: &str,
    ) -> Result<PathBuf, CodeMindPathsError> {
        // Validate caller-controlled input before creating any directories.
        validate_component("repository ID", repository_id)?;
        self.ensure_base_dirs()?;
        let path = self.repository_dir(repository_id)?;
        ensure_private_directory(&path)?;
        Ok(path)
    }

    /// Create and return a repository's scope directory.
    pub fn ensure_repository_scope_dir(
        &self,
        repository_id: &str,
        workspace_key: &str,
    ) -> Result<PathBuf, CodeMindPathsError> {
        // Reject invalid components before creating even the repository root.
        validate_component("repository ID", repository_id)?;
        validate_component("workspace key", workspace_key)?;
        let repository_dir = self.ensure_repository_dir(repository_id)?;
        let scopes_dir = repository_dir.join(REPOSITORY_SCOPES_DIR_NAME);
        ensure_private_directory(&scopes_dir)?;

        let scope_dir = self.repository_scope_dir(repository_id, workspace_key)?;
        ensure_private_directory(&scope_dir)?;
        Ok(scope_dir)
    }

    /// Create and return an operation-owned workspace directory.
    ///
    /// The checkout subdirectory is not created here; the Git layer will own
    /// that step in a later phase. This method does not create a marker file.
    pub fn ensure_workspace_dir(&self, operation_id: &str) -> Result<PathBuf, CodeMindPathsError> {
        validate_component("operation ID", operation_id)?;
        self.ensure_base_dirs()?;
        let path = self.workspace_dir(operation_id)?;
        ensure_private_directory(&path)?;
        Ok(path)
    }
}

fn validate_component<'a>(
    kind: &'static str,
    value: &'a str,
) -> Result<&'a str, CodeMindPathsError> {
    let is_safe = !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');

    if is_safe {
        Ok(value)
    } else {
        Err(CodeMindPathsError::InvalidPathComponent {
            kind,
            value: value.to_owned(),
        })
    }
}

fn ensure_existing_private_directory(path: &Path) -> Result<(), CodeMindPathsError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(CodeMindPathsError::SymlinkDirectory {
                path: path.to_path_buf(),
            });
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err(CodeMindPathsError::NotDirectory {
                path: path.to_path_buf(),
            });
        }
        Ok(_) => {}
        Err(source) => {
            return Err(CodeMindPathsError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    }

    restrict_directory_permissions(path)
}

fn ensure_regular_metadata_file(path: &Path) -> Result<(), CodeMindPathsError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(CodeMindPathsError::SymlinkFile {
            path: path.to_path_buf(),
        }),
        Ok(metadata) if !metadata.is_file() => Err(CodeMindPathsError::NotRegularFile {
            path: path.to_path_buf(),
        }),
        Ok(_) => Ok(()),
        Err(source) => Err(CodeMindPathsError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn ensure_private_directory(path: &Path) -> Result<(), CodeMindPathsError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(CodeMindPathsError::SymlinkDirectory {
                path: path.to_path_buf(),
            });
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err(CodeMindPathsError::NotDirectory {
                path: path.to_path_buf(),
            });
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|source| CodeMindPathsError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        }
        Err(source) => {
            return Err(CodeMindPathsError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    }

    restrict_directory_permissions(path)
}

fn restrict_directory_permissions(path: &Path) -> Result<(), CodeMindPathsError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|source| {
            CodeMindPathsError::Permissions {
                path: path.to_path_buf(),
                source,
            }
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{CodeMindPaths, CodeMindPathsError};
    use std::fs;
    use tempfile::TempDir;

    fn paths_in_temp_home() -> (TempDir, CodeMindPaths) {
        let home = TempDir::new().expect("temporary home directory should be created");
        let paths = CodeMindPaths::from_home_dir(home.path())
            .expect("TempDir should supply an absolute home path");
        (home, paths)
    }

    #[test]
    fn root_is_derived_from_home_not_the_current_working_directory() {
        let (home, paths) = paths_in_temp_home();

        let root = home.path().join(".codemind");
        assert_eq!(paths.root(), root.as_path());
        assert_eq!(paths.repositories_dir(), root.join("repositories"));
        assert_eq!(paths.workspaces_dir(), root.join("workspaces"));
        assert_eq!(paths.config_dir(), root.join("config"));
    }

    #[test]
    fn relative_home_paths_are_rejected() {
        let error = CodeMindPaths::from_home_dir("relative/home")
            .expect_err("relative home paths must not depend on process cwd");

        assert!(matches!(
            error,
            CodeMindPathsError::HomeDirectoryNotAbsolute { .. }
        ));
    }

    #[test]
    fn metadata_paths_are_deterministic_and_match_the_layout() {
        let (home, paths) = paths_in_temp_home();
        let root = home.path().join(".codemind");

        assert_eq!(
            paths.repository_metadata_path("repo_123").unwrap(),
            root.join("repositories")
                .join("repo_123")
                .join("repository.json")
        );
        assert_eq!(
            paths
                .repository_state_path("repo_123", "scope_abc")
                .unwrap(),
            root.join("repositories")
                .join("repo_123")
                .join("scopes")
                .join("scope_abc")
                .join("state.json")
        );
        assert_eq!(
            paths.workspace_marker_path("op_123").unwrap(),
            root.join("workspaces")
                .join("op_123")
                .join("workspace.json")
        );
        assert_eq!(
            paths.workspace_repository_dir("op_123").unwrap(),
            root.join("workspaces").join("op_123").join("repository")
        );
    }

    #[test]
    fn caller_supplied_path_components_cannot_escape_the_root() {
        let (_home, paths) = paths_in_temp_home();

        for value in ["", ".", "..", "../outside", "a/b", "a\\b", "repo id", "x\n"] {
            assert!(paths.repository_dir(value).is_err(), "accepted {value:?}");
            assert!(paths.workspace_dir(value).is_err(), "accepted {value:?}");
            assert!(paths.repository_scope_dir("repo_a", value).is_err());
        }
    }

    #[test]
    fn invalid_identifiers_are_rejected_before_filesystem_mutation() {
        let (_home, paths) = paths_in_temp_home();

        assert!(paths.ensure_repository_dir("../outside").is_err());
        assert!(
            paths
                .ensure_repository_scope_dir("repo_a", "../outside")
                .is_err()
        );
        assert!(paths.ensure_workspace_dir("../outside").is_err());
        assert!(!paths.root().exists());
    }

    #[test]
    fn base_directory_creation_does_not_create_reserved_config_directory() {
        let (_home, paths) = paths_in_temp_home();

        paths
            .ensure_base_dirs()
            .expect("base directories should be created");
        assert!(paths.root().is_dir());
        assert!(paths.repositories_dir().is_dir());
        assert!(paths.workspaces_dir().is_dir());
        assert!(paths.locks_dir().is_dir());
        assert!(!paths.config_dir().exists());
    }

    #[test]
    fn repository_scope_and_workspace_directories_are_created() {
        let (_home, paths) = paths_in_temp_home();

        let repository_dir = paths
            .ensure_repository_dir("repo_a")
            .expect("repository directory should be created");
        let scope_dir = paths
            .ensure_repository_scope_dir("repo_a", "scope_a")
            .expect("scope directory should be created");
        let workspace_dir = paths
            .ensure_workspace_dir("op_a")
            .expect("workspace directory should be created");

        assert!(repository_dir.is_dir());
        assert!(scope_dir.is_dir());
        assert!(workspace_dir.is_dir());
        assert!(!paths.workspace_repository_dir("op_a").unwrap().exists());
        assert!(!paths.workspace_marker_path("op_a").unwrap().exists());
    }

    #[test]
    fn repository_scope_lock_name_is_a_stable_lowercase_sha256_digest() {
        let (_home, paths) = paths_in_temp_home();

        let lock_path = paths.scope_lock_path("repo_a", "scope_a").unwrap();
        let file_name = lock_path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("lock file name should be valid UTF-8");

        assert_eq!(
            file_name,
            "repository-5efc9a3dc44d076e502abd2d89d5e93743e79db47cbd3bd9eb1759ec55dbff6c.lock"
        );
        assert!(!file_name.contains("repo_a"));
        assert!(!file_name.contains("scope_a"));
    }

    #[test]
    fn repository_scope_locks_are_exclusive_and_independent() {
        let (_home, paths) = paths_in_temp_home();
        let first = paths
            .acquire_repository_scope_lock("repo_a", "scope_a")
            .unwrap();

        assert!(
            paths
                .try_acquire_repository_scope_lock("repo_a", "scope_a")
                .unwrap()
                .is_none(),
            "the same scope must be locked across independent file handles"
        );
        assert!(
            paths
                .try_acquire_repository_scope_lock("repo_a", "scope_b")
                .unwrap()
                .is_some(),
            "a different workspace scope must not be blocked"
        );

        drop(first);
        assert!(
            paths
                .try_acquire_repository_scope_lock("repo_a", "scope_a")
                .unwrap()
                .is_some(),
            "dropping the guard must release the OS lock"
        );
    }

    #[cfg(unix)]
    #[test]
    fn repository_scope_lock_rejects_a_symlinked_lock_file() {
        use std::os::unix::fs::symlink;

        let (home, paths) = paths_in_temp_home();
        paths.ensure_base_dirs().unwrap();
        let lock_path = paths.scope_lock_path("repo_a", "scope_a").unwrap();
        let outside = home.path().join("outside.lock");
        fs::write(&outside, b"").unwrap();
        symlink(&outside, &lock_path).unwrap();

        assert!(matches!(
            paths.acquire_repository_scope_lock("repo_a", "scope_a"),
            Err(CodeMindPathsError::SymlinkFile { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn created_directories_have_private_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let (_home, paths) = paths_in_temp_home();
        paths.ensure_base_dirs().unwrap();

        let paths_to_check = vec![
            paths.root().to_path_buf(),
            paths.repositories_dir(),
            paths.workspaces_dir(),
        ];
        for path in &paths_to_check {
            let mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o700, "unexpected permissions for {}", path.display());
        }
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_link_directories_are_rejected() {
        use std::os::unix::fs::symlink;

        let (home, paths) = paths_in_temp_home();
        paths.ensure_base_dirs().unwrap();
        let outside = home.path().join("outside");
        fs::create_dir(&outside).unwrap();

        let workspace_root = paths.workspaces_dir();
        fs::remove_dir(&workspace_root).unwrap();
        symlink(&outside, &workspace_root).unwrap();

        assert!(matches!(
            paths.ensure_base_dirs(),
            Err(CodeMindPathsError::SymlinkDirectory { .. })
        ));
        assert!(!outside.join("unexpected").exists());
    }

    #[test]
    fn non_directory_path_components_are_rejected() {
        let (home, paths) = paths_in_temp_home();
        paths.ensure_base_dirs().unwrap();
        let file = paths.repositories_dir().join("repo_file");
        fs::write(&file, b"not a directory").unwrap();

        assert!(matches!(
            paths.ensure_repository_dir("repo_file"),
            Err(CodeMindPathsError::NotDirectory { .. })
        ));
        assert!(home.path().is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn existing_metadata_reads_reject_symlinked_record_files() {
        use std::os::unix::fs::symlink;

        let (home, paths) = paths_in_temp_home();
        paths.ensure_repository_dir("repo_a").unwrap();
        let outside = home.path().join("outside.json");
        fs::write(&outside, b"{}").unwrap();
        let metadata_path = paths.repository_metadata_path("repo_a").unwrap();
        symlink(&outside, &metadata_path).unwrap();

        assert!(matches!(
            paths.existing_repository_metadata_path("repo_a"),
            Err(CodeMindPathsError::SymlinkFile { .. })
        ));
    }

    #[cfg(unix)]
    #[test]
    fn existing_state_reads_reject_symlinked_scope_directories() {
        use std::os::unix::fs::symlink;

        let (home, paths) = paths_in_temp_home();
        paths
            .ensure_repository_scope_dir("repo_a", "scope_a")
            .unwrap();
        let outside = home.path().join("outside-scope");
        fs::create_dir(&outside).unwrap();
        let scope = paths.repository_scope_dir("repo_a", "scope_a").unwrap();
        fs::remove_dir(&scope).unwrap();
        symlink(&outside, &scope).unwrap();

        assert!(matches!(
            paths.existing_repository_state_path("repo_a", "scope_a"),
            Err(CodeMindPathsError::SymlinkDirectory { .. })
        ));
    }
}

//! Stable identity for remote repositories and path-bound identity for local inputs.
//!
//! Repository identity is independent of a temporary clone path. Remote locators
//! are normalized and validated before being hashed; URLs containing credentials,
//! query strings, or fragments are rejected so they cannot leak into metadata.

use crate::{
    repository::paths::{CodeMindPaths, CodeMindPathsError},
    source::identity::{
        digest_text, repository_id_for_local_path, repository_id_for_stable_identity,
    },
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    net::Ipv6Addr,
    path::{Path, PathBuf},
};
use thiserror::Error;
use uuid::Uuid;

pub const REPOSITORY_METADATA_SCHEMA_VERSION: u32 = 2;
const MAX_LOCATOR_BYTES: usize = 4_096;
const MAX_BRANCH_BYTES: usize = 255;

/// How a stable CodeMind repository identity was obtained.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryIdentityKind {
    /// Identity is derived from a normalized remote Git locator.
    Remote,
    /// Identity is tied to a canonical local directory path because no usable
    /// stable remote locator was available.
    LocalPath,
}

/// Validated repository identity and its non-secret canonical locator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepositoryIdentity {
    repository_id: String,
    /// Stable identity key. The optional `.git` suffix is removed here so
    /// equivalent remote locators resolve to the same CodeMind repository.
    canonical_locator: String,
    /// Credential-free, normalized locator suitable for an actual `git clone`.
    /// This preserves an optional `.git` suffix because some servers require it.
    remote_locator: Option<String>,
    kind: RepositoryIdentityKind,
}

impl RepositoryIdentity {
    /// Create a stable identity from a supported remote Git locator.
    pub fn from_remote(locator: &str) -> Result<Self, RepositoryIdentityError> {
        let canonical_locator = normalize_remote_locator(locator)?;
        let remote_locator = normalize_fetch_locator(locator)?;
        let repository_id = repository_id_for_stable_identity(&canonical_locator);
        Ok(Self {
            repository_id,
            canonical_locator,
            remote_locator: Some(remote_locator),
            kind: RepositoryIdentityKind::Remote,
        })
    }

    /// Create a path-bound identity for an existing local directory.
    ///
    /// This method deliberately does not inspect `.git/config`; Git metadata
    /// inspection belongs to the Git module. Use
    /// [`Self::from_local_directory_with_origin`] when the caller already has a
    /// trusted origin value from a Git-aware layer.
    pub fn from_local_directory(path: impl AsRef<Path>) -> Result<Self, RepositoryIdentityError> {
        Self::from_local_directory_with_origin(path, None)
    }

    /// Prefer a valid origin URL when one is supplied. If the origin is absent
    /// or unusable, fall back to the canonical local path. This allows callers
    /// to pass an origin discovered by Git without making this module parse Git
    /// configuration itself.
    pub fn from_local_directory_with_origin(
        path: impl AsRef<Path>,
        origin: Option<&str>,
    ) -> Result<Self, RepositoryIdentityError> {
        let canonical_root = canonical_directory(path.as_ref())?;
        if let Some(origin) = origin
            && let Ok(identity) = Self::from_remote(origin)
        {
            return Ok(identity);
        }

        let repository_id = repository_id_for_local_path(&canonical_root);
        // The canonical path is used only to derive the stable local identity.
        // Persisting it would disclose the user's directory layout in metadata.
        let canonical_locator = format!("local-path:{repository_id}");
        Ok(Self {
            repository_id,
            canonical_locator,
            remote_locator: None,
            kind: RepositoryIdentityKind::LocalPath,
        })
    }

    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// A normalized remote URL or an opaque local-path identity key. Local
    /// filesystem paths are deliberately not retained in durable metadata.
    pub fn canonical_locator(&self) -> &str {
        &self.canonical_locator
    }

    pub fn kind(&self) -> RepositoryIdentityKind {
        self.kind
    }

    /// Credential-free normalized URL to use for Git transport. Unlike
    /// `canonical_locator`, this preserves a `.git` suffix when supplied.
    pub fn remote_locator(&self) -> Option<&str> {
        self.remote_locator.as_deref()
    }

    fn validate(&self) -> Result<(), RepositoryIdentityError> {
        validate_repository_id(&self.repository_id)?;
        if self.canonical_locator.trim().is_empty()
            || self.canonical_locator.len() > MAX_LOCATOR_BYTES
            || self.canonical_locator.chars().any(char::is_control)
        {
            return Err(RepositoryIdentityError::InvalidMetadata);
        }

        match self.kind {
            RepositoryIdentityKind::Remote => {
                let normalized = normalize_remote_locator(&self.canonical_locator)?;
                let remote_locator = self
                    .remote_locator
                    .as_deref()
                    .ok_or(RepositoryIdentityError::InvalidMetadata)?;
                let normalized_remote = normalize_fetch_locator(remote_locator)?;
                if normalized.as_str() != self.canonical_locator.as_str()
                    || normalized_remote.as_str() != remote_locator
                    || normalize_remote_locator(remote_locator)? != self.canonical_locator
                    || repository_id_for_stable_identity(&normalized) != self.repository_id
                {
                    return Err(RepositoryIdentityError::IdentityMismatch);
                }
            }
            RepositoryIdentityKind::LocalPath => {
                // The original path is intentionally not persisted. The opaque
                // locator binds the metadata record to the already-derived ID.
                let Some(digest) = self.repository_id.strip_prefix("repo-") else {
                    return Err(RepositoryIdentityError::InvalidMetadata);
                };
                if self.remote_locator.is_some()
                    || digest.len() != 64
                    || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                    || self.canonical_locator != format!("local-path:{}", self.repository_id)
                {
                    return Err(RepositoryIdentityError::InvalidMetadata);
                }
            }
        }
        Ok(())
    }
}

/// Stable workspace/security-scope directory key. Raw workspace IDs must never
/// be used as filesystem components because they are external identity values.
pub fn workspace_scope_key(workspace_id: &str) -> Result<String, RepositoryIdentityError> {
    if workspace_id.trim().is_empty()
        || workspace_id.len() > 512
        || workspace_id.chars().any(char::is_control)
    {
        return Err(RepositoryIdentityError::InvalidWorkspaceId);
    }

    Ok(format!(
        "scope-{}",
        digest_text(&format!("codemind-workspace-scope-v1\0{workspace_id}"))
    ))
}

/// Durable repository metadata stored at `repositories/<repository-id>/repository.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RepositoryMetadata {
    pub schema_version: u32,
    pub identity: RepositoryIdentity,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub default_branch: Option<String>,
}

impl RepositoryMetadata {
    pub fn new(identity: RepositoryIdentity) -> Result<Self, RepositoryIdentityError> {
        identity.validate()?;
        let now = Utc::now();
        Ok(Self {
            schema_version: REPOSITORY_METADATA_SCHEMA_VERSION,
            identity,
            created_at: now,
            last_seen_at: now,
            default_branch: None,
        })
    }

    pub fn repository_id(&self) -> &str {
        self.identity.repository_id()
    }

    pub fn touch(&mut self) {
        let now = Utc::now();
        if now > self.last_seen_at {
            self.last_seen_at = now;
        }
    }

    pub fn set_default_branch(
        &mut self,
        branch: Option<&str>,
    ) -> Result<(), RepositoryIdentityError> {
        if let Some(branch) = branch {
            validate_branch_name(branch)?;
        }
        self.default_branch = branch.map(str::to_owned);
        self.validate()
    }

    pub fn validate(&self) -> Result<(), RepositoryIdentityError> {
        if self.schema_version != REPOSITORY_METADATA_SCHEMA_VERSION {
            return Err(RepositoryIdentityError::UnsupportedMetadataVersion {
                actual: self.schema_version,
                supported: REPOSITORY_METADATA_SCHEMA_VERSION,
            });
        }
        self.identity.validate()?;
        if self.last_seen_at < self.created_at {
            return Err(RepositoryIdentityError::InvalidMetadata);
        }
        if let Some(branch) = &self.default_branch {
            validate_branch_name(branch)?;
        }
        Ok(())
    }

    pub fn save(&self, paths: &CodeMindPaths) -> Result<(), RepositoryIdentityError> {
        self.validate()?;
        paths.ensure_repository_dir(self.repository_id())?;
        let destination = paths.repository_metadata_path(self.repository_id())?;
        write_json_atomically(&destination, self)
    }

    pub fn load(
        paths: &CodeMindPaths,
        repository_id: &str,
    ) -> Result<Self, RepositoryIdentityError> {
        validate_repository_id(repository_id)?;
        let path = paths
            .existing_repository_metadata_path(repository_id)
            .map_err(map_paths_error)?;
        let metadata: Self = read_json(&path)?;
        metadata.validate()?;
        if metadata.repository_id() != repository_id {
            return Err(RepositoryIdentityError::IdentityMismatch);
        }
        Ok(metadata)
    }
}

#[derive(Debug, Error)]
pub enum RepositoryIdentityError {
    #[error("repository locator is empty or malformed")]
    InvalidRemoteLocator,
    #[error("remote URL credentials are not accepted; provide a credential-free locator")]
    CredentialBearingRemote,
    #[error("unsupported remote Git scheme `{0}`")]
    UnsupportedRemoteScheme(String),
    #[error("remote locator contains a query or fragment")]
    RemoteQueryOrFragment,
    #[error("remote locator contains an invalid host, port, or repository path")]
    InvalidRemoteComponents,
    #[error("unsupported or malformed remote locator format")]
    UnsupportedRemoteFormat,
    #[error("local repository path could not be canonicalized: {path:?}: {source}")]
    LocalPathIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("local repository path is not a directory: {0:?}")]
    LocalPathNotDirectory(PathBuf),
    #[error("workspace ID is empty, too long, or contains control characters")]
    InvalidWorkspaceId,
    #[error("invalid repository ID")]
    InvalidRepositoryId,
    #[error("invalid or inconsistent repository metadata")]
    InvalidMetadata,
    #[error("repository identity does not match its canonical locator")]
    IdentityMismatch,
    #[error(
        "metadata schema version {actual} is not supported (supported: {supported}); metadata was preserved and will not be replaced without an explicit migration"
    )]
    UnsupportedMetadataVersion { actual: u32, supported: u32 },
    #[error("branch name is empty, too long, or contains control characters")]
    InvalidBranchName,
    #[error("repository metadata could not be read or written at {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("repository metadata serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error(transparent)]
    Paths(#[from] CodeMindPathsError),
}

fn map_paths_error(error: CodeMindPathsError) -> RepositoryIdentityError {
    match error {
        CodeMindPathsError::Io { path, source } => RepositoryIdentityError::Io { path, source },
        other => RepositoryIdentityError::Paths(other),
    }
}

fn validate_branch_name(branch: &str) -> Result<(), RepositoryIdentityError> {
    let invalid = branch.trim().is_empty()
        || branch.len() > MAX_BRANCH_BYTES
        || branch.chars().any(char::is_control)
        || branch.chars().any(char::is_whitespace)
        || branch.contains("..")
        || branch == "@"
        || branch.contains("@{")
        || branch.starts_with('-')
        || branch.ends_with('.')
        || branch
            .chars()
            .any(|character| matches!(character, '~' | '^' | ':' | '?' | '*' | '[' | '\\'))
        || branch.split('/').any(|component| {
            component.is_empty()
                || component.starts_with('.')
                || component.ends_with('.')
                || component.to_ascii_lowercase().ends_with(".lock")
        });
    if invalid {
        return Err(RepositoryIdentityError::InvalidBranchName);
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf, RepositoryIdentityError> {
    let canonical =
        fs::canonicalize(path).map_err(|source| RepositoryIdentityError::LocalPathIo {
            path: path.to_path_buf(),
            source,
        })?;
    let metadata =
        fs::metadata(&canonical).map_err(|source| RepositoryIdentityError::LocalPathIo {
            path: canonical.clone(),
            source,
        })?;
    if !metadata.is_dir() {
        return Err(RepositoryIdentityError::LocalPathNotDirectory(canonical));
    }
    Ok(canonical)
}

fn validate_repository_id(repository_id: &str) -> Result<(), RepositoryIdentityError> {
    if repository_id.is_empty()
        || repository_id.len() > 512
        || repository_id.chars().any(char::is_control)
        || !repository_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(RepositoryIdentityError::InvalidRepositoryId);
    }
    Ok(())
}

/// Strictly normalize supported Git URL forms without retaining secrets.
/// Supported forms are HTTP(S), `ssh://`, `git://`, and the common
/// `git@host:owner/repository.git` SSH form. The first version intentionally
/// rejects arbitrary usernames, URL user-info passwords, query strings and
/// fragments rather than risking a credential leak into persistent metadata.
fn normalize_remote_locator(input: &str) -> Result<String, RepositoryIdentityError> {
    normalize_remote_locator_with_suffix(input, true)
}

fn normalize_fetch_locator(input: &str) -> Result<String, RepositoryIdentityError> {
    normalize_remote_locator_with_suffix(input, false)
}

fn normalize_remote_locator_with_suffix(
    input: &str,
    strip_git_suffix: bool,
) -> Result<String, RepositoryIdentityError> {
    if input.chars().any(char::is_control) {
        return Err(RepositoryIdentityError::InvalidRemoteLocator);
    }
    let value = input.trim();
    if value.is_empty() || value.len() > MAX_LOCATOR_BYTES {
        return Err(RepositoryIdentityError::InvalidRemoteLocator);
    }
    if value.chars().any(char::is_whitespace) {
        return Err(RepositoryIdentityError::InvalidRemoteLocator);
    }
    if value.contains('?') || value.contains('#') {
        return Err(RepositoryIdentityError::RemoteQueryOrFragment);
    }

    let (scheme, user_prefix, authority, path) =
        if let Some((scheme, remainder)) = value.split_once("://") {
            let scheme = scheme.to_ascii_lowercase();
            if !matches!(scheme.as_str(), "http" | "https" | "ssh" | "git") {
                return Err(RepositoryIdentityError::UnsupportedRemoteScheme(scheme));
            }
            let slash = remainder.find('/').unwrap_or(remainder.len());
            let authority = &remainder[..slash];
            let path = remainder.get(slash..).unwrap_or_default();
            if authority.is_empty() || path.is_empty() {
                return Err(RepositoryIdentityError::InvalidRemoteComponents);
            }
            let (user_prefix, authority) = if let Some((user, host)) = authority.rsplit_once('@') {
                if scheme == "ssh" && user == "git" && !host.contains('@') {
                    (Some("git@"), host)
                } else {
                    return Err(RepositoryIdentityError::CredentialBearingRemote);
                }
            } else {
                if authority.contains('@') {
                    return Err(RepositoryIdentityError::CredentialBearingRemote);
                }
                (None, authority)
            };
            (scheme, user_prefix, authority, path)
        } else {
            // Parse the standard GitHub/GitLab SSH locator `git@host:group/repo.git`.
            let (user, remainder) = value
                .split_once('@')
                .ok_or(RepositoryIdentityError::UnsupportedRemoteFormat)?;
            if user != "git" {
                return Err(RepositoryIdentityError::CredentialBearingRemote);
            }
            let (host, path) = remainder
                .split_once(':')
                .ok_or(RepositoryIdentityError::UnsupportedRemoteFormat)?;
            if host.is_empty() || path.is_empty() || host.contains('/') {
                return Err(RepositoryIdentityError::InvalidRemoteComponents);
            }
            ("ssh".to_owned(), Some("git@"), host, path)
        };

    let authority = normalize_authority(&scheme, authority)?;
    let path = normalize_repository_path(path, strip_git_suffix)?;
    let user_prefix = user_prefix.unwrap_or_default();
    Ok(format!("{scheme}://{user_prefix}{authority}/{path}"))
}

fn normalize_authority(scheme: &str, authority: &str) -> Result<String, RepositoryIdentityError> {
    if authority.is_empty() || authority.contains('@') || authority.contains('/') {
        return Err(RepositoryIdentityError::InvalidRemoteComponents);
    }

    let (host, port) = if authority.starts_with('[') {
        let close = authority
            .find(']')
            .ok_or(RepositoryIdentityError::InvalidRemoteComponents)?;
        let literal = &authority[1..close];
        literal
            .parse::<Ipv6Addr>()
            .map_err(|_| RepositoryIdentityError::InvalidRemoteComponents)?;
        let tail = &authority[close + 1..];
        let port = if tail.is_empty() {
            None
        } else if let Some(raw_port) = tail.strip_prefix(':') {
            Some(parse_port(raw_port)?)
        } else {
            return Err(RepositoryIdentityError::InvalidRemoteComponents);
        };
        (format!("[{}]", literal.to_ascii_lowercase()), port)
    } else {
        if authority.matches(':').count() > 1 {
            return Err(RepositoryIdentityError::InvalidRemoteComponents);
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, raw_port)) => (host, Some(parse_port(raw_port)?)),
            None => (authority, None),
        };
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        if !valid_dns_or_ipv4_host(&host) {
            return Err(RepositoryIdentityError::InvalidRemoteComponents);
        }
        (host, port)
    };

    let default_port = match scheme {
        "http" => Some(80),
        "https" => Some(443),
        "ssh" => Some(22),
        "git" => Some(9418),
        _ => None,
    };
    match port {
        Some(port) if Some(port) != default_port => Ok(format!("{host}:{port}")),
        _ => Ok(host),
    }
}

fn parse_port(raw: &str) -> Result<u16, RepositoryIdentityError> {
    if raw.is_empty() || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RepositoryIdentityError::InvalidRemoteComponents);
    }
    let port = raw
        .parse::<u16>()
        .map_err(|_| RepositoryIdentityError::InvalidRemoteComponents)?;
    if port == 0 {
        return Err(RepositoryIdentityError::InvalidRemoteComponents);
    }
    Ok(port)
}

fn valid_dns_or_ipv4_host(host: &str) -> bool {
    if host.is_empty() || host.len() > 253 {
        return false;
    }
    if host.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn normalize_repository_path(
    raw_path: &str,
    strip_git_suffix: bool,
) -> Result<String, RepositoryIdentityError> {
    let trimmed = raw_path.trim_matches('/');
    if trimmed.is_empty() || trimmed.contains('\\') {
        return Err(RepositoryIdentityError::InvalidRemoteComponents);
    }
    let mut segments = Vec::new();
    for segment in trimmed.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(RepositoryIdentityError::InvalidRemoteComponents);
        }
        let lower = segment.to_ascii_lowercase();
        if lower.contains("%2f") || lower.contains("%5c") || lower.contains("%00") {
            return Err(RepositoryIdentityError::InvalidRemoteComponents);
        }
        segments.push(segment.to_owned());
    }
    if segments.len() < 2 {
        return Err(RepositoryIdentityError::InvalidRemoteComponents);
    }

    if strip_git_suffix
        && let Some(last) = segments.last_mut()
        && last.to_ascii_lowercase().ends_with(".git")
    {
        last.truncate(last.len() - 4);
    }
    if segments.last().is_some_and(String::is_empty) {
        return Err(RepositoryIdentityError::InvalidRemoteComponents);
    }
    Ok(segments.join("/"))
}

fn write_json_atomically<T: Serialize>(
    destination: &Path,
    value: &T,
) -> Result<(), RepositoryIdentityError> {
    let parent = destination
        .parent()
        .ok_or(RepositoryIdentityError::InvalidMetadata)?;
    let temporary = parent.join(format!(".metadata-{}.tmp", Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(value)?;
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|source| RepositoryIdentityError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.write_all(&bytes)
            .map_err(|source| RepositoryIdentityError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.sync_all()
            .map_err(|source| RepositoryIdentityError::Io {
                path: temporary.clone(),
                source,
            })?;
        fs::rename(&temporary, destination).map_err(|source| RepositoryIdentityError::Io {
            path: destination.to_path_buf(),
            source,
        })?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, RepositoryIdentityError> {
    let bytes = fs::read(path).map_err(|source| RepositoryIdentityError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}

#[cfg(test)]
mod tests {
    use super::{
        RepositoryIdentity, RepositoryIdentityError, RepositoryIdentityKind, RepositoryMetadata,
        workspace_scope_key,
    };
    use std::{fs, path::Path};
    use tempfile::TempDir;

    #[test]
    fn codemind_github_remote_has_one_stable_identity_across_https_forms() {
        // This is the project's real public GitHub repository. The Level 1
        // test deliberately exercises URL normalization only; it performs no
        // network request and does not depend on GitHub availability.
        let with_git_suffix =
            RepositoryIdentity::from_remote("https://github.com/0xshariq/codemind.git/").unwrap();
        let without_git_suffix =
            RepositoryIdentity::from_remote("https://github.com/0xshariq/codemind").unwrap();

        assert_eq!(with_git_suffix.kind(), RepositoryIdentityKind::Remote);
        assert_eq!(
            with_git_suffix.canonical_locator(),
            "https://github.com/0xshariq/codemind"
        );
        assert_eq!(
            with_git_suffix.repository_id(),
            without_git_suffix.repository_id()
        );
        assert_eq!(
            with_git_suffix.remote_locator(),
            Some("https://github.com/0xshariq/codemind.git")
        );
    }

    #[test]
    fn normalizes_https_host_trailing_slash_and_git_suffix() {
        let a = RepositoryIdentity::from_remote("https://GitHub.COM/owner/repo.git/").unwrap();
        let b = RepositoryIdentity::from_remote("https://github.com/owner/repo").unwrap();
        assert_eq!(a.canonical_locator(), "https://github.com/owner/repo");
        assert_eq!(
            a.remote_locator(),
            Some("https://github.com/owner/repo.git")
        );
        assert_eq!(b.remote_locator(), Some("https://github.com/owner/repo"));
        assert_eq!(a.repository_id(), b.repository_id());
    }

    #[test]
    fn normalizes_default_port_and_ssh_scp_form() {
        let https =
            RepositoryIdentity::from_remote("https://github.com:443/owner/repo.git").unwrap();
        let https_plain = RepositoryIdentity::from_remote("https://github.com/owner/repo").unwrap();
        assert_eq!(https.repository_id(), https_plain.repository_id());

        let scp = RepositoryIdentity::from_remote("git@GitHub.com:owner/repo.git").unwrap();
        let ssh = RepositoryIdentity::from_remote("ssh://git@github.com/owner/repo/").unwrap();
        assert_eq!(scp.canonical_locator(), "ssh://git@github.com/owner/repo");
        assert_eq!(
            scp.remote_locator(),
            Some("ssh://git@github.com/owner/repo.git")
        );
        assert_eq!(scp.repository_id(), ssh.repository_id());
    }

    #[test]
    fn different_remote_repositories_have_different_ids() {
        let first = RepositoryIdentity::from_remote("https://github.com/owner/one").unwrap();
        let second = RepositoryIdentity::from_remote("https://github.com/owner/two").unwrap();
        assert_ne!(first.repository_id(), second.repository_id());
    }

    #[test]
    fn rejects_credentials_queries_fragments_controls_and_unsupported_schemes() {
        for bad in [
            "https://user:token@github.com/owner/repo",
            "https://user@github.com/owner/repo",
            "https://github.com/owner/repo?token=secret",
            "https://github.com/owner/repo#fragment",
            "https://github.com/owner/repo\n",
            "file:///tmp/repo",
            "https://github.com/owner/../repo",
            "https://github.com/owner%2frepo",
        ] {
            assert!(
                RepositoryIdentity::from_remote(bad).is_err(),
                "accepted {bad:?}"
            );
        }
        let error = RepositoryIdentity::from_remote("ftp://example.com/owner/repo").unwrap_err();
        assert!(matches!(
            error,
            RepositoryIdentityError::UnsupportedRemoteScheme(_)
        ));
    }

    #[test]
    fn local_path_identity_is_canonical_and_origin_is_preferred() {
        let home = TempDir::new().unwrap();
        let local = home.path().join("repo");
        fs::create_dir(&local).unwrap();
        let direct = RepositoryIdentity::from_local_directory(&local).unwrap();
        let with_origin = RepositoryIdentity::from_local_directory_with_origin(
            &local,
            Some("https://github.com/owner/repo.git"),
        )
        .unwrap();
        assert_eq!(direct.kind(), RepositoryIdentityKind::LocalPath);
        assert_eq!(with_origin.kind(), RepositoryIdentityKind::Remote);
        assert_ne!(direct.repository_id(), with_origin.repository_id());
        assert_eq!(
            direct.canonical_locator(),
            format!("local-path:{}", direct.repository_id())
        );
        let local_path = local.to_string_lossy();
        assert!(!direct.canonical_locator().contains(local_path.as_ref()));
        assert_eq!(
            RepositoryIdentity::from_local_directory(Path::new(&local))
                .unwrap()
                .repository_id(),
            direct.repository_id()
        );
    }

    #[test]
    fn unusable_origin_falls_back_to_opaque_local_identity_without_persisting_paths_or_secrets() {
        let home = TempDir::new().unwrap();
        let local = home.path().join("repo");
        fs::create_dir(&local).unwrap();
        let identity = RepositoryIdentity::from_local_directory_with_origin(
            &local,
            Some("https://user:secret@example.com/owner/repo"),
        )
        .unwrap();
        assert_eq!(identity.kind(), RepositoryIdentityKind::LocalPath);
        assert!(!identity.canonical_locator().contains("secret"));
        let local_path = local.to_string_lossy();
        assert!(!identity.canonical_locator().contains(local_path.as_ref()));
        let serialized = serde_json::to_string(&identity).unwrap();
        assert!(!serialized.contains(local_path.as_ref()));
    }

    #[test]
    fn workspace_scope_key_is_deterministic_opaque_and_validated() {
        let a = workspace_scope_key("tenant:project:123").unwrap();
        assert_eq!(a, workspace_scope_key("tenant:project:123").unwrap());
        assert!(a.starts_with("scope-"));
        assert!(!a.contains("tenant"));
        assert_ne!(a, workspace_scope_key("tenant:project:456").unwrap());
        assert!(workspace_scope_key("bad\nworkspace").is_err());
    }

    #[test]
    fn metadata_rejects_invalid_default_branch_names() {
        let identity = RepositoryIdentity::from_remote("https://github.com/owner/repo").unwrap();
        let mut metadata = RepositoryMetadata::new(identity).unwrap();
        for branch in [
            "",
            "bad branch",
            "../main",
            "-option",
            "main.lock",
            "topic@{1}",
        ] {
            assert!(
                metadata.set_default_branch(Some(branch)).is_err(),
                "accepted {branch:?}"
            );
        }
        metadata
            .set_default_branch(Some("feature/phase-2"))
            .unwrap();
    }

    #[test]
    fn metadata_does_not_store_credential_bearing_locator() {
        let error =
            RepositoryIdentity::from_remote("https://user:password@example.com/o/r").unwrap_err();
        assert!(matches!(
            error,
            RepositoryIdentityError::CredentialBearingRemote
        ));
    }
}

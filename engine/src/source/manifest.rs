use super::{Language, identity::digest_bytes, limits};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    /// Canonical repository-relative path using `/` separators.
    pub path: String,
    pub language: Option<Language>,
    /// Exact byte length of the content captured into `SourceSnapshot::files`.
    pub bytes: u64,
    /// Lowercase 64-character SHA-256 digest of the captured content.
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceManifest {
    pub repository_id: String,
    pub source_snapshot_id: String,
    pub files: Vec<FileEntry>,
    pub omitted_files: Vec<OmittedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OmittedFile {
    /// Canonical repository-relative path using `/` separators.
    pub path: String,
    /// Best-known byte length from metadata or a bounded capture attempt.
    pub bytes: u64,
    /// Stable, non-empty reason why the source provider did not capture it.
    pub reason: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SourceManifestError {
    #[error("manifest repository and snapshot identities must be non-empty")]
    MissingIdentity,
    #[error("manifest identity field exceeds the canonical context length limit")]
    IdentityTooLong,
    #[error("manifest contains an invalid repository-relative path: {0}")]
    InvalidPath(String),
    #[error("manifest contains a duplicate path: {0}")]
    DuplicatePath(String),
    #[error("manifest file digest is not a lowercase SHA-256 digest for {0}")]
    InvalidDigest(String),
    #[error("manifest language does not match the source extension for {0}")]
    LanguageMismatch(String),
    #[error("manifest exceeds the source layer's file-count limit")]
    TooManyEntries,
    #[error("manifest exceeds the source layer's captured-byte limit")]
    TooManyBytes,
    #[error("manifest omission reason is empty or too long for {0}")]
    InvalidOmissionReason(String),
    #[error("manifest source snapshot ID does not match its canonical identity material")]
    SnapshotIdentityMismatch,
}

impl SourceManifest {
    /// Validate the manifest independently from the snapshot byte map.
    pub fn validate(&self) -> Result<(), SourceManifestError> {
        if self.repository_id.trim().is_empty() || self.source_snapshot_id.trim().is_empty() {
            return Err(SourceManifestError::MissingIdentity);
        }
        if self.repository_id.chars().any(char::is_control)
            || self.source_snapshot_id.chars().any(char::is_control)
        {
            return Err(SourceManifestError::MissingIdentity);
        }
        if self.repository_id.len() > crate::contract::context::MAX_CONTEXT_FIELD_BYTES
            || self.source_snapshot_id.len() > crate::contract::context::MAX_CONTEXT_FIELD_BYTES
        {
            return Err(SourceManifestError::IdentityTooLong);
        }
        if self.files.len().saturating_add(self.omitted_files.len()) > limits::DEFAULT_MAX_FILES {
            return Err(SourceManifestError::TooManyEntries);
        }

        let mut paths = HashSet::with_capacity(self.files.len() + self.omitted_files.len());
        let mut total_captured_bytes = 0_u64;
        for file in &self.files {
            let normalized = limits::normalize_relative_path(Path::new(&file.path))
                .filter(|normalized| normalized == &file.path)
                .ok_or_else(|| SourceManifestError::InvalidPath(file.path.clone()))?;
            if !paths.insert(normalized) {
                return Err(SourceManifestError::DuplicatePath(file.path.clone()));
            }
            if !is_sha256_hex(&file.digest) {
                return Err(SourceManifestError::InvalidDigest(file.path.clone()));
            }
            if Language::from_path(Path::new(&file.path)) != file.language {
                return Err(SourceManifestError::LanguageMismatch(file.path.clone()));
            }
            total_captured_bytes = total_captured_bytes.saturating_add(file.bytes);
        }
        if total_captured_bytes > limits::DEFAULT_MAX_TOTAL_BYTES {
            return Err(SourceManifestError::TooManyBytes);
        }

        for omitted in &self.omitted_files {
            let normalized = limits::normalize_relative_path(Path::new(&omitted.path))
                .filter(|normalized| normalized == &omitted.path)
                .ok_or_else(|| SourceManifestError::InvalidPath(omitted.path.clone()))?;
            if !paths.insert(normalized) {
                return Err(SourceManifestError::DuplicatePath(omitted.path.clone()));
            }
            if omitted.reason.trim().is_empty()
                || omitted.reason.len() > limits::MAX_PATH_BYTES
                || omitted.reason.chars().any(char::is_control)
            {
                return Err(SourceManifestError::InvalidOmissionReason(
                    omitted.path.clone(),
                ));
            }
        }
        if self.recomputed_snapshot_id() != self.source_snapshot_id {
            return Err(SourceManifestError::SnapshotIdentityMismatch);
        }
        Ok(())
    }

    /// Canonical, order-independent material used to identify the captured
    /// source manifest. File content is represented by its SHA-256 digest;
    /// omitted file contents are not claimed to have been captured, but their
    /// path, known size and omission reason affect the snapshot identity.
    pub fn identity_material(&self) -> Vec<u8> {
        let mut material = b"codemind-source-manifest-v2\0".to_vec();
        let mut files: Vec<&FileEntry> = self.files.iter().collect();
        files.sort_by(|left, right| left.path.cmp(&right.path));
        for file in files {
            material.push(b'F');
            append_field(&mut material, file.path.as_bytes());
            material.extend_from_slice(&file.bytes.to_be_bytes());
            append_field(&mut material, file.digest.as_bytes());
        }

        let mut omitted: Vec<&OmittedFile> = self.omitted_files.iter().collect();
        omitted.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then(left.reason.cmp(&right.reason))
        });
        for item in omitted {
            material.push(b'O');
            append_field(&mut material, item.path.as_bytes());
            material.extend_from_slice(&item.bytes.to_be_bytes());
            append_field(&mut material, item.reason.as_bytes());
        }
        material
    }

    /// Verify that the ID was computed from this manifest's accepted and
    /// omitted inputs. The prefix is part of the public source contract.
    pub fn recomputed_snapshot_id(&self) -> String {
        format!("snapshot-{}", digest_bytes(&self.identity_material()))
    }
}

fn append_field(output: &mut Vec<u8>, field: &[u8]) {
    output.extend_from_slice(&(field.len() as u64).to_be_bytes());
    output.extend_from_slice(field);
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{FileEntry, OmittedFile, SourceManifest, SourceManifestError};
    use crate::source::Language;
    use crate::source::identity::digest_text;

    fn manifest() -> SourceManifest {
        SourceManifest {
            repository_id: "repo-1".into(),
            source_snapshot_id: "snapshot-pending".into(),
            files: vec![
                FileEntry {
                    path: "src/lib.rs".into(),
                    language: Some(Language::Rust),
                    bytes: 3,
                    digest: digest_text("abc"),
                },
                FileEntry {
                    path: "src/mod.rs".into(),
                    language: Some(Language::Rust),
                    bytes: 2,
                    digest: digest_text("xy"),
                },
            ],
            omitted_files: vec![
                OmittedFile {
                    path: "large.bin".into(),
                    bytes: 200,
                    reason: "file exceeds max_file_bytes".into(),
                },
                OmittedFile {
                    path: "large2.bin".into(),
                    bytes: 300,
                    reason: "file exceeds max_file_bytes".into(),
                },
            ],
        }
    }

    #[test]
    fn manifest_identity_includes_omissions_and_is_order_independent() {
        let mut first = manifest();
        first.source_snapshot_id = first.recomputed_snapshot_id();
        let mut second = first.clone();
        second.files.reverse();
        second.omitted_files.reverse();
        assert_eq!(
            first.recomputed_snapshot_id(),
            second.recomputed_snapshot_id()
        );
        second.omitted_files[0].bytes += 1;
        assert_ne!(
            first.recomputed_snapshot_id(),
            second.recomputed_snapshot_id()
        );
        assert!(first.validate().is_ok());
    }

    #[test]
    fn manifest_rejects_duplicate_and_noncanonical_paths() {
        let mut value = manifest();
        value.source_snapshot_id = value.recomputed_snapshot_id();
        value.omitted_files[0].path = "src/../lib.rs".into();
        assert!(matches!(
            value.validate(),
            Err(SourceManifestError::InvalidPath(_))
        ));

        let mut value = manifest();
        value.omitted_files[0].path = "src/lib.rs".into();
        assert!(matches!(
            value.validate(),
            Err(SourceManifestError::DuplicatePath(_))
        ));
    }

    #[test]
    fn manifest_rejects_a_stale_snapshot_identity() {
        let mut value = manifest();
        value.source_snapshot_id = "snapshot-stale".into();
        assert_eq!(
            value.validate(),
            Err(SourceManifestError::SnapshotIdentityMismatch)
        );
    }
}

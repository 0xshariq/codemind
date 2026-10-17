use super::{
    SourceManifest,
    identity::{digest_bytes, graph_version_for_snapshot},
    limits,
};
pub use crate::contract::context::RepositoryContext;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io, path::Path};
use thiserror::Error;

/// An immutable-by-convention capture of source bytes and the metadata used to
/// identify them. Consumers must use `file_bytes`/`file_text`, not reread
/// `root`, so the graph is derived from the same bytes represented by the ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSnapshot {
    /// Canonical local root retained for provenance/debugging only. It is not
    /// used to load files after capture and must not be treated as a global ID.
    pub root: std::path::PathBuf,
    pub context: RepositoryContext,
    pub manifest: SourceManifest,
    /// Exact bytes that were captured and hashed. Callers should treat this map
    /// and its associated manifest/context as one immutable value.
    pub files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SnapshotValidationError {
    #[error("invalid repository context: {0}")]
    Context(String),
    #[error("source manifest is invalid: {0}")]
    Manifest(String),
    #[error("source snapshot context does not match manifest identities")]
    ContextManifestMismatch,
    #[error("source snapshot ID does not match manifest content")]
    SnapshotIdentityMismatch,
    #[error("graph version does not match the source snapshot and projection revision")]
    GraphVersionMismatch,
    #[error("captured byte map does not match the manifest file set")]
    FileSetMismatch,
    #[error("manifest path was not safely normalized: {0}")]
    UnsafePath(String),
    #[error("captured content length/digest mismatch for {0}")]
    ContentMismatch(String),
}

impl SourceSnapshot {
    pub fn file_bytes(&self, path: &str) -> io::Result<&[u8]> {
        let path_obj = Path::new(path);
        let normalized = limits::normalize_relative_path(path_obj).filter(|value| value == path);
        if normalized.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "snapshot path must be a normalized repository-relative path",
            ));
        }
        self.files.get(path).map(Vec::as_slice).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("snapshot file not found: {path}"),
            )
        })
    }

    pub fn file_text(&self, path: &str) -> io::Result<&str> {
        std::str::from_utf8(self.file_bytes(path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    /// Validate the link among context, manifest, snapshot ID, graph projection
    /// version, and captured bytes before graph publication.
    pub fn validate(&self) -> Result<(), SnapshotValidationError> {
        self.context
            .validate()
            .map_err(|error| SnapshotValidationError::Context(error.to_string()))?;
        self.manifest
            .validate()
            .map_err(|error| SnapshotValidationError::Manifest(error.to_string()))?;

        if self.manifest.repository_id != self.context.repository_id
            || self.manifest.source_snapshot_id != self.context.source_snapshot_id
        {
            return Err(SnapshotValidationError::ContextManifestMismatch);
        }
        if self.manifest.recomputed_snapshot_id() != self.context.source_snapshot_id {
            return Err(SnapshotValidationError::SnapshotIdentityMismatch);
        }
        if graph_version_for_snapshot(&self.context.source_snapshot_id)
            != self.context.graph_version
        {
            return Err(SnapshotValidationError::GraphVersionMismatch);
        }
        if self.files.len() != self.manifest.files.len() {
            return Err(SnapshotValidationError::FileSetMismatch);
        }

        for entry in &self.manifest.files {
            if limits::normalize_relative_path(Path::new(&entry.path)).as_deref()
                != Some(entry.path.as_str())
            {
                return Err(SnapshotValidationError::UnsafePath(entry.path.clone()));
            }
            let Some(content) = self.files.get(&entry.path) else {
                return Err(SnapshotValidationError::FileSetMismatch);
            };
            if content.len() as u64 != entry.bytes || digest_bytes(content) != entry.digest {
                return Err(SnapshotValidationError::ContentMismatch(entry.path.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{RepositoryContext, SnapshotValidationError, SourceSnapshot};
    use crate::source::{
        FileEntry, Language, SourceManifest,
        identity::{digest_bytes, graph_version_for_snapshot},
    };
    use std::{collections::BTreeMap, path::PathBuf};

    fn snapshot() -> SourceSnapshot {
        let bytes = b"fn main() {}".to_vec();
        let mut manifest = SourceManifest {
            repository_id: "repo-1".into(),
            source_snapshot_id: String::new(),
            files: vec![FileEntry {
                path: "src/main.rs".into(),
                language: Some(Language::Rust),
                bytes: bytes.len() as u64,
                digest: digest_bytes(&bytes),
            }],
            omitted_files: Vec::new(),
        };
        manifest.source_snapshot_id = manifest.recomputed_snapshot_id();
        let context = RepositoryContext {
            repository_id: manifest.repository_id.clone(),
            workspace_id: "workspace-1".into(),
            source_snapshot_id: manifest.source_snapshot_id.clone(),
            graph_version: graph_version_for_snapshot(&manifest.source_snapshot_id),
        };
        SourceSnapshot {
            root: PathBuf::from("/not-needed-after-capture"),
            context,
            manifest,
            files: BTreeMap::from([("src/main.rs".into(), bytes)]),
        }
    }

    #[test]
    fn validates_snapshot_context_manifest_and_pinned_bytes() {
        assert!(snapshot().validate().is_ok());
        assert_eq!(snapshot().file_text("src/main.rs").unwrap(), "fn main() {}");
    }

    #[test]
    fn rejects_mutated_captured_bytes() {
        let mut value = snapshot();
        value
            .files
            .insert("src/main.rs".into(), b"fn test() {}".to_vec());
        assert!(matches!(
            value.validate(),
            Err(SnapshotValidationError::ContentMismatch(_))
        ));
    }

    #[test]
    fn rejects_noncanonical_path_lookup() {
        let value = snapshot();
        assert_eq!(
            value.file_bytes("src/../main.rs").unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
}

use super::{context::RepositoryContext, error::ContractError, result::Completeness};
use crate::source::{SourceManifest, SourceSnapshot, limits};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositorySourceRequest {
    pub workspace_id: String,
    pub root: String,
    pub max_file_bytes: u64,
}

impl RepositorySourceRequest {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.workspace_id.trim().is_empty() {
            return Err(ContractError::InvalidInput(
                "workspace_id is required".to_owned(),
            ));
        }
        if self.workspace_id.len() > 512 || self.workspace_id.chars().any(char::is_control) {
            return Err(ContractError::InvalidInput(
                "workspace_id is malformed or too long".to_owned(),
            ));
        }
        if self.root.trim().is_empty()
            || self.root.len() > 4_096
            || self.root.chars().any(char::is_control)
        {
            return Err(ContractError::InvalidInput(
                "repository root is empty or malformed".to_owned(),
            ));
        }
        if self.max_file_bytes == 0 || self.max_file_bytes > limits::DEFAULT_MAX_TOTAL_BYTES {
            return Err(ContractError::LimitExceeded(format!(
                "max_file_bytes must be between 1 and {}",
                limits::DEFAULT_MAX_TOTAL_BYTES
            )));
        }
        Ok(())
    }
}

/// Metadata returned after source capture. Source bytes remain in
/// `SourceSnapshot`; this contract deliberately carries only the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositorySourceResult {
    pub context: RepositoryContext,
    pub manifest: SourceManifest,
    pub completeness: Completeness,
    pub diagnostics: Vec<String>,
    pub limitations: Vec<String>,
}

impl RepositorySourceResult {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.context.validate()?;
        if self.manifest.repository_id != self.context.repository_id
            || self.manifest.source_snapshot_id != self.context.source_snapshot_id
        {
            return Err(ContractError::InvalidContext(
                "source manifest identity does not match repository context".to_owned(),
            ));
        }

        if self.manifest.files.len() > limits::DEFAULT_MAX_FILES
            || self.manifest.omitted_files.len() > limits::DEFAULT_MAX_FILES
        {
            return Err(ContractError::LimitExceeded(
                "source manifest exceeds the bounded file/omission count".to_owned(),
            ));
        }
        let mut paths =
            HashSet::with_capacity(self.manifest.files.len() + self.manifest.omitted_files.len());
        for file in &self.manifest.files {
            let path = Path::new(&file.path);
            if file.path.trim().is_empty()
                || file.path.contains('\\')
                || !limits::is_safe_relative(path)
                || file.digest.trim().is_empty()
            {
                return Err(ContractError::InvalidInput(
                    "source manifest contains an invalid path or file digest".to_owned(),
                ));
            }
            if !paths.insert(file.path.as_str()) {
                return Err(ContractError::Conflict(format!(
                    "duplicate source manifest path: {}",
                    file.path
                )));
            }
        }
        for omitted in &self.manifest.omitted_files {
            let path = Path::new(&omitted.path);
            if omitted.path.trim().is_empty()
                || omitted.path.contains('\\')
                || !limits::is_safe_relative(path)
                || omitted.reason.trim().is_empty()
            {
                return Err(ContractError::InvalidInput(
                    "source manifest contains invalid omission metadata".to_owned(),
                ));
            }
            if !paths.insert(omitted.path.as_str()) {
                return Err(ContractError::Conflict(format!(
                    "source path appears more than once in the manifest: {}",
                    omitted.path
                )));
            }
        }
        if !self.manifest.omitted_files.is_empty() && self.completeness == Completeness::Complete {
            return Err(ContractError::InvalidInput(
                "source result cannot be complete when files were omitted".to_owned(),
            ));
        }
        if self.diagnostics.len() > 100 || self.limitations.len() > 100 {
            return Err(ContractError::LimitExceeded(
                "source result metadata exceeds the bounded contract".to_owned(),
            ));
        }
        if self
            .diagnostics
            .iter()
            .chain(self.limitations.iter())
            .any(|entry| {
                entry.trim().is_empty() || entry.len() > super::result::MAX_METADATA_ITEM_BYTES
            })
        {
            return Err(ContractError::InvalidInput(
                "source result diagnostic/limitation is empty or too long".to_owned(),
            ));
        }
        Ok(())
    }
}

impl From<&SourceSnapshot> for RepositorySourceResult {
    fn from(snapshot: &SourceSnapshot) -> Self {
        let omitted_count = snapshot.manifest.omitted_files.len();
        let completeness = if omitted_count == 0 {
            Completeness::Complete
        } else {
            Completeness::Truncated
        };
        // The manifest retains every omission. Metadata uses a bounded sample
        // plus an aggregate message so a large repository cannot exceed the
        // canonical diagnostics/limitations contract just by having many files.
        let mut omissions: Vec<String> = snapshot
            .manifest
            .omitted_files
            .iter()
            .take(if omitted_count > super::result::MAX_DIAGNOSTICS {
                super::result::MAX_DIAGNOSTICS - 1
            } else {
                super::result::MAX_DIAGNOSTICS
            })
            .map(|file| format!("{}: {}", file.path, file.reason))
            .collect();
        if omitted_count > super::result::MAX_DIAGNOSTICS {
            omissions.push(format!(
                "{} additional omitted files are listed in the source manifest",
                omitted_count - (super::result::MAX_DIAGNOSTICS - 1)
            ));
        }
        Self {
            context: snapshot.context.clone(),
            manifest: snapshot.manifest.clone(),
            completeness,
            diagnostics: omissions.clone(),
            limitations: omissions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RepositorySourceRequest;

    #[test]
    fn source_request_requires_workspace_root_and_positive_bounded_file_limit() {
        let request = RepositorySourceRequest {
            workspace_id: "workspace".into(),
            root: ".".into(),
            max_file_bytes: 1024,
        };
        assert!(request.validate().is_ok());
        let invalid = RepositorySourceRequest {
            max_file_bytes: 0,
            ..request
        };
        assert!(invalid.validate().is_err());
    }
}

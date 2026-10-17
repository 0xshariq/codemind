use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum encoded length for one opaque identity in a repository context.
/// Context identifiers are not paths and must not be normalized by trimming.
pub const MAX_CONTEXT_FIELD_BYTES: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RepositoryContext {
    pub repository_id: String,
    pub workspace_id: String,
    pub source_snapshot_id: String,
    pub graph_version: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContextError {
    #[error("repository context field `{0}` is empty")]
    EmptyField(&'static str),
    #[error("repository context field `{field}` exceeds {max_bytes} bytes")]
    FieldTooLong {
        field: &'static str,
        max_bytes: usize,
    },
    #[error("repository context field `{0}` contains a control character")]
    ControlCharacter(&'static str),
    #[error("repository context field `{0}` does not match the expected context")]
    Mismatch(&'static str),
}

impl RepositoryContext {
    /// Validate the shape of all four independent context identities.
    ///
    /// This checks input shape only; it does not authorize access to the
    /// workspace, repository, snapshot, or graph.
    pub fn validate(&self) -> Result<(), ContextError> {
        let fields = [
            ("repository_id", self.repository_id.as_str()),
            ("workspace_id", self.workspace_id.as_str()),
            ("source_snapshot_id", self.source_snapshot_id.as_str()),
            ("graph_version", self.graph_version.as_str()),
        ];

        for (field, value) in fields {
            if value.trim().is_empty() {
                return Err(ContextError::EmptyField(field));
            }
            if value.len() > MAX_CONTEXT_FIELD_BYTES {
                return Err(ContextError::FieldTooLong {
                    field,
                    max_bytes: MAX_CONTEXT_FIELD_BYTES,
                });
            }
            if value.chars().any(char::is_control) {
                return Err(ContextError::ControlCharacter(field));
            }
        }
        Ok(())
    }

    /// Validate this context and require exact identity equality with `expected`.
    /// The method deliberately does not infer compatibility across snapshots or
    /// graph versions: callers must use the context they actually requested.
    pub fn ensure_matches(&self, expected: &Self) -> Result<(), ContextError> {
        self.validate()?;
        expected.validate()?;

        for (field, actual, expected) in [
            (
                "repository_id",
                self.repository_id.as_str(),
                expected.repository_id.as_str(),
            ),
            (
                "workspace_id",
                self.workspace_id.as_str(),
                expected.workspace_id.as_str(),
            ),
            (
                "source_snapshot_id",
                self.source_snapshot_id.as_str(),
                expected.source_snapshot_id.as_str(),
            ),
            (
                "graph_version",
                self.graph_version.as_str(),
                expected.graph_version.as_str(),
            ),
        ] {
            if actual != expected {
                return Err(ContextError::Mismatch(field));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ContextError, RepositoryContext};

    fn valid_context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo-1".into(),
            workspace_id: "workspace-1".into(),
            source_snapshot_id: "snapshot-1".into(),
            graph_version: "graph-1".into(),
        }
    }

    #[test]
    fn accepts_a_complete_context_without_collapsing_identity_fields() {
        assert!(valid_context().validate().is_ok());
    }

    #[test]
    fn rejects_empty_and_control_character_identity_fields() {
        let mut context = valid_context();
        context.workspace_id = "  ".into();
        assert_eq!(
            context.validate(),
            Err(ContextError::EmptyField("workspace_id"))
        );

        let mut context = valid_context();
        context.graph_version = "graph\n1".into();
        assert_eq!(
            context.validate(),
            Err(ContextError::ControlCharacter("graph_version"))
        );
    }

    #[test]
    fn exact_context_match_includes_snapshot_and_graph_version() {
        let actual = valid_context();
        let mut expected = actual.clone();
        expected.source_snapshot_id = "snapshot-2".into();
        assert_eq!(
            actual.ensure_matches(&expected),
            Err(ContextError::Mismatch("source_snapshot_id"))
        );
    }
}

use crate::engines::coie::model::{EntityKind, SourceEvidence};
use crate::source::RepositoryContext;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeSelectionKind {
    Symbol,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet {
    pub id: String,
    pub context: RepositoryContext,
    pub selected_entity_id: String,
    pub selection_kind: ChangeSelectionKind,
    pub source: Option<SourceEvidence>,
    pub analysis_run_id: Option<String>,
}

impl ChangeSet {
    pub fn new(
        id: impl Into<String>,
        context: RepositoryContext,
        selected_entity_id: impl Into<String>,
        selection_kind: ChangeSelectionKind,
        source: Option<SourceEvidence>,
    ) -> Result<Self, String> {
        let selected_entity_id = selected_entity_id.into();
        let id = id.into();
        if id.trim().is_empty() || selected_entity_id.trim().is_empty() {
            return Err("change set identity is required".to_owned());
        }
        Ok(Self {
            id,
            context,
            selected_entity_id,
            selection_kind,
            source,
            analysis_run_id: None,
        })
    }

    pub fn for_entity(
        id: impl Into<String>,
        context: RepositoryContext,
        entity_id: impl Into<String>,
        kind: EntityKind,
        source: Option<SourceEvidence>,
    ) -> Result<Self, String> {
        let selection_kind = match kind {
            EntityKind::Symbol => ChangeSelectionKind::Symbol,
            EntityKind::File => ChangeSelectionKind::File,
            _ => return Err("change selection must reference a symbol or file".to_owned()),
        };
        Self::new(id, context, entity_id, selection_kind, source)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisRun {
    pub id: String,
    pub change_set_id: String,
    pub context: RepositoryContext,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
}

impl AnalysisRun {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty()
            || self.change_set_id.trim().is_empty()
            || self.status.trim().is_empty()
            || self.started_at.trim().is_empty()
        {
            return Err("analysis run identity and lifecycle fields are required".to_owned());
        }
        self.context.validate().map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::RepositoryContext;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn change_selection_accepts_only_symbol_and_file_entities() {
        assert!(
            ChangeSet::for_entity("set", context(), "symbol", EntityKind::Symbol, None).is_ok()
        );
        assert!(ChangeSet::for_entity("set", context(), "file", EntityKind::File, None).is_ok());
        assert!(
            ChangeSet::for_entity("set", context(), "directory", EntityKind::Directory, None)
                .is_err()
        );
        assert!(
            AnalysisRun {
                id: "run".into(),
                change_set_id: "set".into(),
                context: context(),
                status: "completed".into(),
                started_at: "now".into(),
                completed_at: None
            }
            .validate()
            .is_ok()
        );
    }
}

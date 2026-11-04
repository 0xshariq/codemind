use crate::engines::coie::model::{EntityKind, SourceEvidence};
use crate::source::RepositoryContext;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeSelectionKind {
    Symbol,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ChangeInputOrigin {
    #[default]
    SelectedEntity,
    GitCommit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitChangeProvenance {
    pub base_commit: String,
    pub target_commit: String,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet {
    pub id: String,
    pub context: RepositoryContext,
    pub selected_entity_id: String,
    pub selection_kind: ChangeSelectionKind,
    pub source: Option<SourceEvidence>,
    #[serde(default)]
    pub origin: ChangeInputOrigin,
    #[serde(default)]
    pub git_provenance: Option<GitChangeProvenance>,
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
            origin: ChangeInputOrigin::SelectedEntity,
            git_provenance: None,
            analysis_run_id: None,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() || self.selected_entity_id.trim().is_empty() {
            return Err("change set identity is required".to_owned());
        }
        self.context.validate().map_err(|error| error.to_string())?;
        match (&self.origin, &self.git_provenance) {
            (ChangeInputOrigin::SelectedEntity, Some(_)) => {
                Err("selected-entity change set cannot carry Git provenance".to_owned())
            }
            (ChangeInputOrigin::GitCommit, None) => {
                Err("Git-origin change set requires commit provenance".to_owned())
            }
            (ChangeInputOrigin::GitCommit, Some(provenance)) => {
                if provenance.base_commit.trim().is_empty()
                    || provenance.target_commit.trim().is_empty()
                    || provenance.changed_paths.is_empty()
                    || provenance
                        .changed_paths
                        .iter()
                        .any(|path| path.trim().is_empty())
                {
                    return Err("Git change provenance is incomplete".to_owned());
                }
                Ok(())
            }
            (ChangeInputOrigin::SelectedEntity, None) => Ok(()),
        }
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

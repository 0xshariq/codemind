//! Typed, language-agnostic records for CodeMind's repository-scoped History Graph.
//!
//! This model intentionally does not use `RepositoryContext` as its storage
//! identity because that type includes a source snapshot and graph projection
//! version. Git history belongs to `(workspace_id, repository_id)` and evolves
//! across snapshots. `HistoryGraphContext::from_source_context` is a one-way
//! projection for callers that have a validated source context.

use crate::{
    contract::context::{MAX_CONTEXT_FIELD_BYTES, RepositoryContext},
    source::limits::MAX_PATH_BYTES,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use thiserror::Error;

pub const HISTORY_GRAPH_SCHEMA_VERSION: u32 = 1;
type HistoryRelationshipKey = (
    HistoryRelationshipKind,
    String,
    String,
    Option<HistoryPathRole>,
);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct HistoryGraphContext {
    pub workspace_id: String,
    pub repository_id: String,
}

impl HistoryGraphContext {
    pub fn new(
        workspace_id: impl Into<String>,
        repository_id: impl Into<String>,
    ) -> Result<Self, HistoryGraphError> {
        let context = Self {
            workspace_id: workspace_id.into(),
            repository_id: repository_id.into(),
        };
        context.validate()?;
        Ok(context)
    }

    /// Project the durable repository scope from a source context. Snapshot
    /// identity and graph projection version are deliberately not copied.
    pub fn from_source_context(context: &RepositoryContext) -> Result<Self, HistoryGraphError> {
        context
            .validate()
            .map_err(|error| HistoryGraphError::InvalidSourceContext(error.to_string()))?;
        Self::new(context.workspace_id.clone(), context.repository_id.clone())
    }

    pub fn validate(&self) -> Result<(), HistoryGraphError> {
        validate_context_field("workspace_id", &self.workspace_id)?;
        validate_context_field("repository_id", &self.repository_id)?;
        Ok(())
    }
}

fn validate_context_field(field: &'static str, value: &str) -> Result<(), HistoryGraphError> {
    if value.trim().is_empty() {
        return Err(HistoryGraphError::InvalidContextField(field));
    }
    if value.len() > MAX_CONTEXT_FIELD_BYTES {
        return Err(HistoryGraphError::ContextFieldTooLong(field));
    }
    if value.chars().any(char::is_control) {
        return Err(HistoryGraphError::ContextFieldContainsControlCharacter(
            field,
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum HistoryEntityKind {
    Repository,
    Commit,
    Directory,
    File,
    Change,
}

/// A safe display form for a repository path plus exact bytes when the path
/// contains non-UTF-8 bytes or display-escaped characters. `raw_bytes_hex` is
/// lowercase hex of the exact raw repository path when display is not lossless.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct HistoryPath {
    pub display: String,
    pub raw_bytes_hex: Option<String>,
}

impl HistoryPath {
    pub fn validate(&self) -> Result<(), HistoryGraphError> {
        let bytes = match &self.raw_bytes_hex {
            Some(hex) => {
                if hex.bytes().any(|byte| byte.is_ascii_uppercase()) {
                    return Err(HistoryGraphError::InvalidRawPathHex);
                }
                let decoded = decode_hex(hex).ok_or(HistoryGraphError::InvalidRawPathHex)?;
                if display_path_from_bytes(&decoded) != self.display
                    || self.display.as_bytes() == decoded.as_slice()
                {
                    return Err(HistoryGraphError::InvalidRawPathHex);
                }
                decoded
            }
            None => {
                let bytes = self.display.as_bytes().to_vec();
                if display_path_from_bytes(&bytes) != self.display {
                    return Err(HistoryGraphError::InvalidRawPathHex);
                }
                bytes
            }
        };
        validate_path_bytes(&bytes, &self.display)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HistoryChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Unmerged,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum HistoryPathRole {
    Old,
    New,
}

/// A tagged entity payload keeps commit, path and change-specific fields typed
/// instead of placing many unrelated optional fields on one universal record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "details", rename_all = "snake_case")]
pub enum HistoryEntityData {
    Repository {
        repository_id: String,
    },
    Commit {
        commit_id: String,
        parent_commit_ids: Vec<String>,
        committed_at_unix_seconds: Option<i64>,
        author: Option<String>,
        subject: Option<String>,
        metadata_complete: bool,
        changed_paths_complete: Option<bool>,
        omitted_paths_at_least: u64,
        /// True for a parent referenced by a scanned commit but outside the
        /// scanned history window. It is a boundary node, not full metadata.
        is_boundary_stub: bool,
    },
    Directory {
        path: HistoryPath,
    },
    File {
        path: HistoryPath,
    },
    Change {
        commit_id: String,
        change_kind: HistoryChangeKind,
        status_code: String,
        old_path: Option<HistoryPath>,
        new_path: Option<HistoryPath>,
    },
}

impl HistoryEntityData {
    pub fn entity_kind(&self) -> HistoryEntityKind {
        match self {
            Self::Repository { .. } => HistoryEntityKind::Repository,
            Self::Commit { .. } => HistoryEntityKind::Commit,
            Self::Directory { .. } => HistoryEntityKind::Directory,
            Self::File { .. } => HistoryEntityKind::File,
            Self::Change { .. } => HistoryEntityKind::Change,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HistoryEntity {
    pub id: String,
    pub context: HistoryGraphContext,
    pub data: HistoryEntityData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum HistoryRelationshipKind {
    ContainsCommit,
    ParentCommit,
    RecordsChange,
    ChangesPath,
    ContainsPath,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HistoryRelationship {
    pub id: String,
    pub kind: HistoryRelationshipKind,
    pub from: String,
    pub to: String,
    pub context: HistoryGraphContext,
    /// Only `ChangesPath` relationships have a role, disambiguating the old
    /// path from the new path for a rename or copy.
    pub path_role: Option<HistoryPathRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HistoryGraph {
    pub schema_version: u32,
    pub context: HistoryGraphContext,
    pub head_commit: String,
    pub entities: Vec<HistoryEntity>,
    pub relationships: Vec<HistoryRelationship>,
    /// False when Git history/path output was bounded or incomplete. Consumers
    /// must not present a partial graph as complete history.
    pub complete: bool,
    pub omitted_commits_at_least: u64,
    pub omitted_path_changes_at_least: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HistoryGraphStats {
    pub entity_count: usize,
    pub repository_count: usize,
    pub commit_count: usize,
    pub boundary_commit_count: usize,
    pub directory_count: usize,
    pub file_count: usize,
    pub change_count: usize,
    pub relationship_count: usize,
    pub complete: bool,
    pub omitted_commits_at_least: u64,
    pub omitted_path_changes_at_least: u64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HistoryGraphError {
    #[error("source context cannot be projected into history context: {0}")]
    InvalidSourceContext(String),
    #[error("history context field `{0}` is empty")]
    InvalidContextField(&'static str),
    #[error("history context field `{0}` exceeds the context length limit")]
    ContextFieldTooLong(&'static str),
    #[error("history context field `{0}` contains a control character")]
    ContextFieldContainsControlCharacter(&'static str),
    #[error("Git history contains no commits")]
    EmptyHistory,
    #[error("Git history head commit `{0}` is not included in the scanned commits")]
    HeadCommitMissing(String),
    #[error("Git history contains an invalid commit ID: `{0}`")]
    InvalidCommitId(String),
    #[error("Git history contains duplicate commit ID: `{0}`")]
    DuplicateCommitId(String),
    #[error("Git history change for commit `{0}` has no valid old or new path")]
    InvalidChangeRecord(String),
    #[error("history contains an invalid repository-relative path: `{0}`")]
    InvalidPath(String),
    #[error("history path has malformed raw-byte hex")]
    InvalidRawPathHex,
    #[error("history graph contains an empty entity or relationship ID")]
    EmptyGraphIdentity,
    #[error("history graph contains duplicate entity ID: `{0}`")]
    DuplicateEntityId(String),
    #[error("history graph contains duplicate `{kind:?}` path entity for `{path}`")]
    DuplicatePathEntity {
        kind: HistoryEntityKind,
        path: String,
    },
    #[error("history graph contains duplicate relationship ID: `{0}`")]
    DuplicateRelationshipId(String),
    #[error("history graph entity `{0}` has a context outside the graph scope")]
    EntityContextMismatch(String),
    #[error("history graph relationship `{0}` has a context outside the graph scope")]
    RelationshipContextMismatch(String),
    #[error(
        "history graph relationship `{relationship_id}` references missing endpoint `{endpoint_id}`"
    )]
    DanglingRelationship {
        relationship_id: String,
        endpoint_id: String,
    },
    #[error("history graph relationship `{0}` has invalid endpoint kinds or path role")]
    InvalidRelationshipShape(String),
    #[error("history graph contains an invalid entity payload for `{0}`")]
    InvalidEntityData(String),
    #[error("history graph relationship `{0}` does not agree with its typed endpoint data")]
    InvalidRelationshipSemantics(String),
    #[error("history graph change `{0}` is missing its commit/path relationship")]
    MissingChangeRelationship(String),
    #[error("history graph schema version {0} is not supported")]
    UnsupportedSchemaVersion(u32),
    #[error("history graph head commit `{0}` has no complete commit entity")]
    InvalidHeadEntity(String),
    #[error("history graph completeness metadata contradicts its commit/path records")]
    InvalidCompleteness,
    #[error("commit `{commit_id}` is missing its parent relationship to `{parent_id}`")]
    MissingParentRelationship {
        commit_id: String,
        parent_id: String,
    },
}

impl HistoryGraph {
    /// Validate graph scope, identities, paths, typed edge endpoints and
    /// commit ancestry before a later storage adapter persists this graph.
    /// Index maps keep validation bounded even for large retained histories.
    pub fn validate(&self) -> Result<(), HistoryGraphError> {
        self.context.validate()?;
        if self.schema_version != HISTORY_GRAPH_SCHEMA_VERSION {
            return Err(HistoryGraphError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        validate_commit_id(&self.head_commit)?;

        let mut entities_by_id: BTreeMap<&str, &HistoryEntity> = BTreeMap::new();
        let mut commits_by_sha: BTreeMap<&str, &HistoryEntity> = BTreeMap::new();
        let mut paths_by_kind: HashMap<(HistoryEntityKind, HistoryPath), &HistoryEntity> =
            HashMap::new();
        let mut repository_roots = Vec::new();
        for entity in &self.entities {
            if entity.id.trim().is_empty() {
                return Err(HistoryGraphError::EmptyGraphIdentity);
            }
            if entity.context != self.context {
                return Err(HistoryGraphError::EntityContextMismatch(entity.id.clone()));
            }
            validate_entity_data(&self.context, entity)?;
            if entities_by_id.insert(entity.id.as_str(), entity).is_some() {
                return Err(HistoryGraphError::DuplicateEntityId(entity.id.clone()));
            }
            match &entity.data {
                HistoryEntityData::Repository { repository_id } => {
                    repository_roots.push((entity.id.as_str(), repository_id.as_str()));
                }
                HistoryEntityData::Commit { commit_id, .. } => {
                    if commits_by_sha.insert(commit_id.as_str(), entity).is_some() {
                        return Err(HistoryGraphError::DuplicateCommitId(commit_id.clone()));
                    }
                }
                HistoryEntityData::Directory { path } => {
                    if paths_by_kind
                        .insert((HistoryEntityKind::Directory, path.clone()), entity)
                        .is_some()
                    {
                        return Err(HistoryGraphError::DuplicatePathEntity {
                            kind: HistoryEntityKind::Directory,
                            path: path.display.clone(),
                        });
                    }
                }
                HistoryEntityData::File { path } => {
                    if paths_by_kind
                        .insert((HistoryEntityKind::File, path.clone()), entity)
                        .is_some()
                    {
                        return Err(HistoryGraphError::DuplicatePathEntity {
                            kind: HistoryEntityKind::File,
                            path: path.display.clone(),
                        });
                    }
                }
                HistoryEntityData::Change { .. } => {}
            }
        }

        if repository_roots.len() != 1
            || repository_roots[0].1 != self.context.repository_id.as_str()
        {
            return Err(HistoryGraphError::InvalidEntityData(
                "repository-root".to_owned(),
            ));
        }
        let root_id = repository_roots[0].0;

        let mut relationships_by_id = BTreeMap::new();
        let mut relationship_keys: HashSet<HistoryRelationshipKey> = HashSet::new();
        for relationship in &self.relationships {
            if relationship.id.trim().is_empty()
                || relationship.from.trim().is_empty()
                || relationship.to.trim().is_empty()
            {
                return Err(HistoryGraphError::EmptyGraphIdentity);
            }
            if relationship.context != self.context {
                return Err(HistoryGraphError::RelationshipContextMismatch(
                    relationship.id.clone(),
                ));
            }
            let from = entities_by_id
                .get(relationship.from.as_str())
                .ok_or_else(|| HistoryGraphError::DanglingRelationship {
                    relationship_id: relationship.id.clone(),
                    endpoint_id: relationship.from.clone(),
                })?;
            let to = entities_by_id
                .get(relationship.to.as_str())
                .ok_or_else(|| HistoryGraphError::DanglingRelationship {
                    relationship_id: relationship.id.clone(),
                    endpoint_id: relationship.to.clone(),
                })?;
            if !valid_relationship_shape(relationship, from, to) {
                return Err(HistoryGraphError::InvalidRelationshipShape(
                    relationship.id.clone(),
                ));
            }
            if !valid_relationship_semantics(relationship, from, to) {
                return Err(HistoryGraphError::InvalidRelationshipSemantics(
                    relationship.id.clone(),
                ));
            }
            if relationships_by_id
                .insert(relationship.id.as_str(), relationship)
                .is_some()
            {
                return Err(HistoryGraphError::DuplicateRelationshipId(
                    relationship.id.clone(),
                ));
            }
            relationship_keys.insert((
                relationship.kind.clone(),
                relationship.from.clone(),
                relationship.to.clone(),
                relationship.path_role.clone(),
            ));
        }

        // Every commit node (including boundary stubs) belongs to this scoped
        // repository root. Parent relationships are checked through indexes,
        // not repeated scans of all entities/edges.
        for entity in entities_by_id.values() {
            if matches!(&entity.data, HistoryEntityData::Commit { .. })
                && !relationship_keys.contains(&(
                    HistoryRelationshipKind::ContainsCommit,
                    root_id.to_owned(),
                    entity.id.clone(),
                    None,
                ))
            {
                return Err(HistoryGraphError::InvalidEntityData(entity.id.clone()));
            }
        }

        let Some(head_entity) = commits_by_sha.get(self.head_commit.as_str()) else {
            return Err(HistoryGraphError::InvalidHeadEntity(
                self.head_commit.clone(),
            ));
        };
        if !matches!(
            &head_entity.data,
            HistoryEntityData::Commit {
                metadata_complete: true,
                is_boundary_stub: false,
                ..
            }
        ) {
            return Err(HistoryGraphError::InvalidHeadEntity(
                self.head_commit.clone(),
            ));
        }

        for entity in entities_by_id.values() {
            match &entity.data {
                HistoryEntityData::Commit {
                    commit_id,
                    parent_commit_ids,
                    metadata_complete: true,
                    ..
                } => {
                    for parent_id in parent_commit_ids {
                        let parent_entity =
                            commits_by_sha.get(parent_id.as_str()).ok_or_else(|| {
                                HistoryGraphError::MissingParentRelationship {
                                    commit_id: commit_id.clone(),
                                    parent_id: parent_id.clone(),
                                }
                            })?;
                        if !relationship_keys.contains(&(
                            HistoryRelationshipKind::ParentCommit,
                            entity.id.clone(),
                            parent_entity.id.clone(),
                            None,
                        )) {
                            return Err(HistoryGraphError::MissingParentRelationship {
                                commit_id: commit_id.clone(),
                                parent_id: parent_id.clone(),
                            });
                        }
                    }
                }
                HistoryEntityData::Change {
                    commit_id,
                    old_path,
                    new_path,
                    ..
                } => {
                    let Some(commit_entity) = commits_by_sha.get(commit_id.as_str()) else {
                        return Err(HistoryGraphError::MissingChangeRelationship(
                            entity.id.clone(),
                        ));
                    };
                    if !relationship_keys.contains(&(
                        HistoryRelationshipKind::RecordsChange,
                        commit_entity.id.clone(),
                        entity.id.clone(),
                        None,
                    )) {
                        return Err(HistoryGraphError::MissingChangeRelationship(
                            entity.id.clone(),
                        ));
                    }
                    for (role, path) in [
                        (HistoryPathRole::Old, old_path.as_ref()),
                        (HistoryPathRole::New, new_path.as_ref()),
                    ] {
                        if let Some(path) = path {
                            let file_entity = paths_by_kind
                                .get(&(HistoryEntityKind::File, path.clone()))
                                .ok_or_else(|| {
                                    HistoryGraphError::MissingChangeRelationship(entity.id.clone())
                                })?;
                            if !relationship_keys.contains(&(
                                HistoryRelationshipKind::ChangesPath,
                                entity.id.clone(),
                                file_entity.id.clone(),
                                Some(role),
                            )) {
                                return Err(HistoryGraphError::MissingChangeRelationship(
                                    entity.id.clone(),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if self.complete
            && (self.omitted_commits_at_least > 0
                || self.omitted_path_changes_at_least > 0
                || self.entities.iter().any(|entity| {
                    matches!(
                        &entity.data,
                        HistoryEntityData::Commit {
                            is_boundary_stub: true,
                            ..
                        }
                    ) || matches!(
                        &entity.data,
                        HistoryEntityData::Commit {
                            changed_paths_complete: Some(false),
                            ..
                        }
                    ) || matches!(
                        &entity.data,
                        HistoryEntityData::Commit {
                            omitted_paths_at_least,
                            ..
                        } if *omitted_paths_at_least > 0
                    )
                }))
        {
            return Err(HistoryGraphError::InvalidCompleteness);
        }
        Ok(())
    }

    pub fn stats(&self) -> HistoryGraphStats {
        let mut stats = HistoryGraphStats {
            entity_count: self.entities.len(),
            repository_count: 0,
            commit_count: 0,
            boundary_commit_count: 0,
            directory_count: 0,
            file_count: 0,
            change_count: 0,
            relationship_count: self.relationships.len(),
            complete: self.complete,
            omitted_commits_at_least: self.omitted_commits_at_least,
            omitted_path_changes_at_least: self.omitted_path_changes_at_least,
        };
        for entity in &self.entities {
            match &entity.data {
                HistoryEntityData::Repository { .. } => stats.repository_count += 1,
                HistoryEntityData::Commit {
                    is_boundary_stub, ..
                } => {
                    stats.commit_count += 1;
                    if *is_boundary_stub {
                        stats.boundary_commit_count += 1;
                    }
                }
                HistoryEntityData::Directory { .. } => stats.directory_count += 1,
                HistoryEntityData::File { .. } => stats.file_count += 1,
                HistoryEntityData::Change { .. } => stats.change_count += 1,
            }
        }
        stats
    }
}

fn validate_entity_data(
    context: &HistoryGraphContext,
    entity: &HistoryEntity,
) -> Result<(), HistoryGraphError> {
    match &entity.data {
        HistoryEntityData::Repository { repository_id } => {
            if repository_id != &context.repository_id {
                return Err(HistoryGraphError::InvalidEntityData(entity.id.clone()));
            }
        }
        HistoryEntityData::Commit {
            commit_id,
            parent_commit_ids,
            metadata_complete,
            committed_at_unix_seconds,
            author,
            subject,
            changed_paths_complete,
            is_boundary_stub,
            ..
        } => {
            validate_commit_id(commit_id)?;
            let mut unique_parents = HashSet::new();
            for parent in parent_commit_ids {
                validate_commit_id(parent)?;
                if parent == commit_id || !unique_parents.insert(parent.as_str()) {
                    return Err(HistoryGraphError::InvalidEntityData(entity.id.clone()));
                }
            }
            if *metadata_complete
                && (committed_at_unix_seconds.is_none()
                    || author.is_none()
                    || subject.is_none()
                    || changed_paths_complete.is_none()
                    || *is_boundary_stub)
            {
                return Err(HistoryGraphError::InvalidEntityData(entity.id.clone()));
            }
            if !*metadata_complete
                && (committed_at_unix_seconds.is_some()
                    || author.is_some()
                    || subject.is_some()
                    || changed_paths_complete.is_some()
                    || !*is_boundary_stub
                    || !parent_commit_ids.is_empty())
            {
                return Err(HistoryGraphError::InvalidEntityData(entity.id.clone()));
            }
        }
        HistoryEntityData::Directory { path } | HistoryEntityData::File { path } => {
            path.validate()?;
        }
        HistoryEntityData::Change {
            commit_id,
            status_code,
            old_path,
            new_path,
            ..
        } => {
            validate_commit_id(commit_id)?;
            if status_code.trim().is_empty() || (old_path.is_none() && new_path.is_none()) {
                return Err(HistoryGraphError::InvalidEntityData(entity.id.clone()));
            }
            if let Some(path) = old_path {
                path.validate()?;
            }
            if let Some(path) = new_path {
                path.validate()?;
            }
        }
    }
    Ok(())
}

fn valid_relationship_shape(
    relationship: &HistoryRelationship,
    from: &HistoryEntity,
    to: &HistoryEntity,
) -> bool {
    match &relationship.kind {
        HistoryRelationshipKind::ContainsCommit => {
            relationship.path_role.is_none()
                && matches!(&from.data, HistoryEntityData::Repository { .. })
                && matches!(&to.data, HistoryEntityData::Commit { .. })
        }
        HistoryRelationshipKind::ParentCommit => {
            relationship.path_role.is_none()
                && matches!(&from.data, HistoryEntityData::Commit { .. })
                && matches!(&to.data, HistoryEntityData::Commit { .. })
        }
        HistoryRelationshipKind::RecordsChange => {
            relationship.path_role.is_none()
                && matches!(&from.data, HistoryEntityData::Commit { .. })
                && matches!(&to.data, HistoryEntityData::Change { .. })
        }
        HistoryRelationshipKind::ChangesPath => {
            relationship.path_role.is_some()
                && matches!(&from.data, HistoryEntityData::Change { .. })
                && matches!(&to.data, HistoryEntityData::File { .. })
        }
        HistoryRelationshipKind::ContainsPath => {
            relationship.path_role.is_none()
                && matches!(
                    &from.data,
                    HistoryEntityData::Repository { .. } | HistoryEntityData::Directory { .. }
                )
                && matches!(
                    &to.data,
                    HistoryEntityData::Directory { .. } | HistoryEntityData::File { .. }
                )
        }
    }
}

fn valid_relationship_semantics(
    relationship: &HistoryRelationship,
    from: &HistoryEntity,
    to: &HistoryEntity,
) -> bool {
    match (
        &relationship.kind,
        &from.data,
        &to.data,
        relationship.path_role.as_ref(),
    ) {
        (
            HistoryRelationshipKind::ParentCommit,
            HistoryEntityData::Commit {
                parent_commit_ids, ..
            },
            HistoryEntityData::Commit {
                commit_id: parent_id,
                ..
            },
            None,
        ) => parent_commit_ids.contains(parent_id),
        (
            HistoryRelationshipKind::RecordsChange,
            HistoryEntityData::Commit {
                commit_id,
                metadata_complete: true,
                is_boundary_stub: false,
                ..
            },
            HistoryEntityData::Change {
                commit_id: change_commit_id,
                ..
            },
            None,
        ) => commit_id == change_commit_id,
        (
            HistoryRelationshipKind::ChangesPath,
            HistoryEntityData::Change { old_path, .. },
            HistoryEntityData::File { path },
            Some(HistoryPathRole::Old),
        ) => old_path.as_ref() == Some(path),
        (
            HistoryRelationshipKind::ChangesPath,
            HistoryEntityData::Change { new_path, .. },
            HistoryEntityData::File { path },
            Some(HistoryPathRole::New),
        ) => new_path.as_ref() == Some(path),
        (
            HistoryRelationshipKind::ContainsPath,
            HistoryEntityData::Repository { .. },
            HistoryEntityData::Directory { path } | HistoryEntityData::File { path },
            None,
        ) => path_bytes_for_validation(path).is_some_and(|bytes| !bytes.contains(&b'/')),
        (
            HistoryRelationshipKind::ContainsPath,
            HistoryEntityData::Directory { path: parent },
            HistoryEntityData::Directory { path: child } | HistoryEntityData::File { path: child },
            None,
        ) => {
            match (
                path_bytes_for_validation(parent),
                path_bytes_for_validation(child),
            ) {
                (Some(parent), Some(child)) if child.len() > parent.len() => {
                    child.starts_with(&parent)
                        && child.get(parent.len()) == Some(&b'/')
                        && !child[parent.len() + 1..].contains(&b'/')
                }
                _ => false,
            }
        }
        // Shape validation has already ensured these endpoint types and the
        // repository context agrees; they need no additional payload check.
        (HistoryRelationshipKind::ContainsCommit, _, _, None) => true,
        _ => false,
    }
}

fn path_bytes_for_validation(path: &HistoryPath) -> Option<Vec<u8>> {
    match &path.raw_bytes_hex {
        Some(hex) => decode_hex(hex),
        None => Some(path.display.as_bytes().to_vec()),
    }
}

pub(crate) fn validate_commit_id(value: &str) -> Result<(), HistoryGraphError> {
    if !matches!(value.len(), 40 | 64)
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(HistoryGraphError::InvalidCommitId(value.to_owned()));
    }
    Ok(())
}

/// Create a stable, control-safe display representation without losing the
/// original path bytes. Backslashes and control code points are escaped so a
/// filename cannot inject line breaks into logs, diagnostics or graph viewers.
pub(crate) fn display_path_from_bytes(bytes: &[u8]) -> String {
    let lossy = String::from_utf8_lossy(bytes);
    let mut display = String::with_capacity(lossy.len());
    for character in lossy.chars() {
        if character == '\\' {
            display.push_str("\\\\");
        } else if character.is_control() {
            use std::fmt::Write as _;
            let _ = write!(&mut display, "\\u{{{:04x}}}", character as u32);
        } else {
            display.push(character);
        }
    }
    display
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

pub(crate) fn validate_path_bytes(bytes: &[u8], display: &str) -> Result<(), HistoryGraphError> {
    let invalid = bytes.is_empty()
        || bytes.len() > MAX_PATH_BYTES
        || bytes[0] == b'/'
        || bytes.contains(&0)
        || bytes
            .split(|byte| *byte == b'/')
            .any(|component| component.is_empty() || component == b"." || component == b"..");
    if invalid || display.is_empty() {
        return Err(HistoryGraphError::InvalidPath(display.to_owned()));
    }
    Ok(())
}

pub(crate) fn decode_hex(value: &str) -> Option<Vec<u8>> {
    // This decoder is used for repository-relative paths only. Apply the path
    // bound before allocating so an externally deserialized graph cannot make
    // validation allocate an unbounded buffer from a forged hex string.
    if value.is_empty() || !value.len().is_multiple_of(2) || value.len() / 2 > MAX_PATH_BYTES {
        return None;
    }
    let mut output = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        output.push((high << 4) | low);
    }
    Some(output)
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod level1_tests {
    use super::{
        decode_hex, display_path_from_bytes, encode_hex, validate_commit_id, validate_path_bytes,
    };

    #[test]
    fn commit_ids_accept_only_lowercase_sha1_or_sha256_hex() {
        assert!(validate_commit_id(&"a".repeat(40)).is_ok());
        assert!(validate_commit_id(&"b".repeat(64)).is_ok());

        assert!(validate_commit_id(&"A".repeat(40)).is_err());
        assert!(validate_commit_id(&"a".repeat(39)).is_err());
        assert!(validate_commit_id(&"g".repeat(40)).is_err());
        assert!(validate_commit_id(&"a".repeat(41)).is_err());
    }

    #[test]
    fn raw_path_hex_round_trips_arbitrary_bytes_and_rejects_malformed_input() {
        let original = [b'a', b'/', 0xff, b'.'];
        let encoded = encode_hex(&original);
        assert_eq!(encoded, "612fff2e");
        assert_eq!(decode_hex(&encoded).as_deref(), Some(original.as_slice()));
        assert_eq!(decode_hex("612FFF2E").as_deref(), Some(original.as_slice()));

        assert!(decode_hex("").is_none());
        assert!(decode_hex("0").is_none());
        assert!(decode_hex("zz").is_none());
    }

    #[test]
    fn display_path_escapes_backslashes_and_control_characters() {
        assert_eq!(
            display_path_from_bytes(b"src/a\\b\n.rs"),
            "src/a\\\\b\\u{000a}.rs"
        );
        assert_eq!(display_path_from_bytes("src/é.rs".as_bytes()), "src/é.rs");
    }

    #[test]
    fn repository_paths_reject_absolute_empty_traversal_and_nul_components() {
        for invalid in [
            &b""[..],
            b"/absolute",
            b"a//b",
            b"a/./b",
            b"a/../b",
            b"a\0b",
        ] {
            assert!(validate_path_bytes(invalid, "display").is_err());
        }
        assert!(validate_path_bytes(b"src/main.rs", "src/main.rs").is_ok());
        assert!(validate_path_bytes(b"src/\xff.rs", "src/�.rs").is_ok());
    }
}

//! Convert raw Git facts into the repository-scoped History Graph. This
//! module never executes Git commands and never persists graph data; the Git
//! boundary supplies facts and a later storage phase owns persistence and
//! verification. Graph-record IDs are random SHA-256 values scoped to one
//! projection build. Git commit IDs remain their native stable identifiers.

use crate::{
    git::{GitChangeKind, GitCommit, GitHistory, GitPath},
    source::identity::RandomIdGenerator,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::model::{
    HISTORY_GRAPH_SCHEMA_VERSION, HistoryChangeKind, HistoryEntity, HistoryEntityData,
    HistoryGraph, HistoryGraphContext, HistoryGraphError, HistoryPath, HistoryPathRole,
    HistoryRelationship, HistoryRelationshipKind, decode_hex, display_path_from_bytes, encode_hex,
    validate_path_bytes,
};

pub fn build_history_graph(
    context: HistoryGraphContext,
    history: &GitHistory,
) -> Result<HistoryGraph, HistoryGraphError> {
    context.validate()?;
    if history.commits.is_empty() {
        return Err(HistoryGraphError::EmptyHistory);
    }

    let head_commit = normalize_commit_id(&history.head_commit)?;
    let mut commits_by_id: BTreeMap<String, &GitCommit> = BTreeMap::new();
    for commit in &history.commits {
        let commit_id = normalize_commit_id(&commit.commit_id)?;
        if commits_by_id.insert(commit_id.clone(), commit).is_some() {
            return Err(HistoryGraphError::DuplicateCommitId(commit_id));
        }
        for parent in &commit.parent_commit_ids {
            let normalized_parent = normalize_commit_id(parent)?;
            if normalized_parent == commit_id {
                return Err(HistoryGraphError::InvalidEntityData(commit_id));
            }
        }
    }
    if !commits_by_id.contains_key(&head_commit) {
        return Err(HistoryGraphError::HeadCommitMissing(head_commit));
    }

    let mut boundary_commit_ids = BTreeSet::new();
    for commit in commits_by_id.values() {
        for parent_id in normalized_parent_ids(commit)? {
            if !commits_by_id.contains_key(&parent_id) {
                boundary_commit_ids.insert(parent_id);
            }
        }
    }

    let mut id_generator = RandomIdGenerator::default();
    let mut semantic_relationships = HashSet::new();
    let mut path_entity_ids: HashMap<(bool, Vec<u8>), String> = HashMap::new();
    let mut commit_entity_ids = HashMap::new();
    for commit_id in commits_by_id
        .keys()
        .cloned()
        .chain(boundary_commit_ids.iter().cloned())
    {
        commit_entity_ids
            .entry(commit_id)
            .or_insert_with(|| id_generator.next("history-entity", "commit"));
    }

    let mut entities: BTreeMap<String, HistoryEntity> = BTreeMap::new();
    let mut relationships: BTreeMap<String, HistoryRelationship> = BTreeMap::new();
    let repository_entity_id = id_generator.next("history-entity", "repository");
    insert_entity(
        &mut entities,
        HistoryEntity {
            id: repository_entity_id.clone(),
            context: context.clone(),
            data: HistoryEntityData::Repository {
                repository_id: context.repository_id.clone(),
            },
        },
    )?;

    // Add all scanned commits, then boundary nodes for parent commits outside
    // the retained window. IDs are minted once and reused for every edge.
    for (commit_id, commit) in &commits_by_id {
        let parent_commit_ids = normalized_parent_ids(commit)?;
        insert_entity(
            &mut entities,
            HistoryEntity {
                id: commit_entity_ids
                    .get(commit_id)
                    .expect("scanned commit has a generated graph identity")
                    .clone(),
                context: context.clone(),
                data: HistoryEntityData::Commit {
                    commit_id: commit_id.clone(),
                    parent_commit_ids,
                    committed_at_unix_seconds: Some(commit.committed_at_unix_seconds),
                    author: Some(commit.author.clone()),
                    subject: Some(commit.subject.clone()),
                    metadata_complete: true,
                    changed_paths_complete: Some(commit.changed_paths_complete),
                    omitted_paths_at_least: if commit.changed_paths_complete {
                        commit.omitted_paths_at_least
                    } else {
                        commit.omitted_paths_at_least.max(1)
                    },
                    is_boundary_stub: false,
                },
            },
        )?;
    }

    for commit_id in &boundary_commit_ids {
        insert_entity(
            &mut entities,
            HistoryEntity {
                id: commit_entity_ids
                    .get(commit_id)
                    .expect("boundary commit has a generated graph identity")
                    .clone(),
                context: context.clone(),
                data: HistoryEntityData::Commit {
                    commit_id: commit_id.clone(),
                    parent_commit_ids: Vec::new(),
                    committed_at_unix_seconds: None,
                    author: None,
                    subject: None,
                    metadata_complete: false,
                    changed_paths_complete: None,
                    omitted_paths_at_least: 0,
                    is_boundary_stub: true,
                },
            },
        )?;
    }

    let all_commit_ids: Vec<String> = commits_by_id
        .keys()
        .cloned()
        .chain(boundary_commit_ids.iter().cloned())
        .collect();
    for commit_id in &all_commit_ids {
        add_relationship(
            &mut relationships,
            &mut id_generator,
            &mut semantic_relationships,
            &context,
            HistoryRelationshipKind::ContainsCommit,
            &repository_entity_id,
            commit_entity_ids
                .get(commit_id)
                .expect("all commit IDs have generated graph identities"),
            None,
        )?;
    }

    let mut seen_changes = HashSet::new();
    for (commit_id, commit) in &commits_by_id {
        let from_id = commit_entity_ids
            .get(commit_id)
            .expect("scanned commit has a generated graph identity")
            .clone();
        for parent_id in normalized_parent_ids(commit)? {
            add_relationship(
                &mut relationships,
                &mut id_generator,
                &mut semantic_relationships,
                &context,
                HistoryRelationshipKind::ParentCommit,
                &from_id,
                commit_entity_ids
                    .get(&parent_id)
                    .expect("parent commit has a generated graph identity"),
                None,
            )?;
        }

        for change in &commit.changed_paths {
            let old_path = change
                .old_path
                .as_ref()
                .map(history_path_from_git)
                .transpose()?;
            let new_path = change
                .new_path
                .as_ref()
                .map(history_path_from_git)
                .transpose()?;
            if old_path.is_none() && new_path.is_none() {
                return Err(HistoryGraphError::InvalidChangeRecord(commit_id.clone()));
            }

            let old_path_bytes = change.old_path.as_ref().map(git_path_bytes).transpose()?;
            let new_path_bytes = change.new_path.as_ref().map(git_path_bytes).transpose()?;
            let change_kind = history_change_kind(&change.kind);
            let change_key = (
                commit_id.clone(),
                change.status_code.clone(),
                change_kind_identity(&change.kind),
                old_path_bytes.clone(),
                new_path_bytes.clone(),
            );
            if !seen_changes.insert(change_key) {
                continue;
            }

            let change_entity_id = id_generator.next("history-entity", "change");
            insert_entity(
                &mut entities,
                HistoryEntity {
                    id: change_entity_id.clone(),
                    context: context.clone(),
                    data: HistoryEntityData::Change {
                        commit_id: commit_id.clone(),
                        change_kind,
                        status_code: change.status_code.clone(),
                        old_path: old_path.clone(),
                        new_path: new_path.clone(),
                    },
                },
            )?;
            add_relationship(
                &mut relationships,
                &mut id_generator,
                &mut semantic_relationships,
                &context,
                HistoryRelationshipKind::RecordsChange,
                &from_id,
                &change_entity_id,
                None,
            )?;

            if let (Some(path), Some(path_bytes)) = (old_path, old_path_bytes) {
                let file_id = add_path_and_directories(
                    &mut entities,
                    &mut relationships,
                    &mut id_generator,
                    &mut semantic_relationships,
                    &mut path_entity_ids,
                    &context,
                    &repository_entity_id,
                    &path,
                    &path_bytes,
                )?;
                add_relationship(
                    &mut relationships,
                    &mut id_generator,
                    &mut semantic_relationships,
                    &context,
                    HistoryRelationshipKind::ChangesPath,
                    &change_entity_id,
                    &file_id,
                    Some(HistoryPathRole::Old),
                )?;
            }
            if let (Some(path), Some(path_bytes)) = (new_path, new_path_bytes) {
                let file_id = add_path_and_directories(
                    &mut entities,
                    &mut relationships,
                    &mut id_generator,
                    &mut semantic_relationships,
                    &mut path_entity_ids,
                    &context,
                    &repository_entity_id,
                    &path,
                    &path_bytes,
                )?;
                add_relationship(
                    &mut relationships,
                    &mut id_generator,
                    &mut semantic_relationships,
                    &context,
                    HistoryRelationshipKind::ChangesPath,
                    &change_entity_id,
                    &file_id,
                    Some(HistoryPathRole::New),
                )?;
            }
        }
    }

    let omitted_path_changes_at_least = history
        .commits
        .iter()
        .map(|commit| {
            if commit.changed_paths_complete {
                commit.omitted_paths_at_least
            } else {
                commit.omitted_paths_at_least.max(1)
            }
        })
        .fold(0_u64, u64::saturating_add);
    let graph = HistoryGraph {
        schema_version: HISTORY_GRAPH_SCHEMA_VERSION,
        context,
        head_commit,
        entities: entities.into_values().collect(),
        relationships: relationships.into_values().collect(),
        complete: history.complete
            && history
                .commits
                .iter()
                .all(|commit| commit.changed_paths_complete),
        omitted_commits_at_least: history
            .omitted_commits_at_least
            .max(boundary_commit_ids.len() as u64),
        omitted_path_changes_at_least,
    };
    graph.validate()?;
    Ok(graph)
}

#[allow(clippy::too_many_arguments)]
fn add_path_and_directories(
    entities: &mut BTreeMap<String, HistoryEntity>,
    relationships: &mut BTreeMap<String, HistoryRelationship>,
    id_generator: &mut RandomIdGenerator,
    semantic_relationships: &mut HashSet<(
        HistoryRelationshipKind,
        String,
        String,
        Option<HistoryPathRole>,
    )>,
    path_entity_ids: &mut HashMap<(bool, Vec<u8>), String>,
    context: &HistoryGraphContext,
    repository_entity_id: &str,
    file_path: &HistoryPath,
    file_path_bytes: &[u8],
) -> Result<String, HistoryGraphError> {
    let components: Vec<&[u8]> = file_path_bytes.split(|byte| *byte == b'/').collect();
    let mut parent_entity_id = repository_entity_id.to_owned();
    let mut prefix: Vec<u8> = Vec::new();

    for component in components.iter().take(components.len().saturating_sub(1)) {
        if !prefix.is_empty() {
            prefix.push(b'/');
        }
        prefix.extend_from_slice(component);
        let directory_path = history_path_from_bytes(&prefix)?;
        let directory_id = path_entity_ids
            .entry((false, prefix.clone()))
            .or_insert_with(|| id_generator.next("history-entity", "directory"))
            .clone();
        insert_entity(
            entities,
            HistoryEntity {
                id: directory_id.clone(),
                context: context.clone(),
                data: HistoryEntityData::Directory {
                    path: directory_path,
                },
            },
        )?;
        add_relationship(
            relationships,
            id_generator,
            semantic_relationships,
            context,
            HistoryRelationshipKind::ContainsPath,
            &parent_entity_id,
            &directory_id,
            None,
        )?;
        parent_entity_id = directory_id;
    }

    let represented_path_bytes = path_bytes(file_path)?;
    if file_path_bytes != represented_path_bytes.as_slice() {
        return Err(HistoryGraphError::InvalidPath(file_path.display.clone()));
    }
    let file_id = path_entity_ids
        .entry((true, file_path_bytes.to_vec()))
        .or_insert_with(|| id_generator.next("history-entity", "file"))
        .clone();
    insert_entity(
        entities,
        HistoryEntity {
            id: file_id.clone(),
            context: context.clone(),
            data: HistoryEntityData::File {
                path: file_path.clone(),
            },
        },
    )?;
    add_relationship(
        relationships,
        id_generator,
        semantic_relationships,
        context,
        HistoryRelationshipKind::ContainsPath,
        &parent_entity_id,
        &file_id,
        None,
    )?;
    Ok(file_id)
}

fn path_bytes(path: &HistoryPath) -> Result<Vec<u8>, HistoryGraphError> {
    let bytes = match &path.raw_bytes_hex {
        Some(hex) => decode_hex(hex).ok_or(HistoryGraphError::InvalidRawPathHex)?,
        None => path.display.as_bytes().to_vec(),
    };
    validate_path_bytes(&bytes, &path.display)?;
    Ok(bytes)
}

fn git_path_bytes(path: &GitPath) -> Result<Vec<u8>, HistoryGraphError> {
    let bytes = match &path.raw_bytes_hex {
        Some(hex) => {
            if hex.bytes().any(|byte| byte.is_ascii_uppercase()) {
                return Err(HistoryGraphError::InvalidRawPathHex);
            }
            let decoded = decode_hex(hex).ok_or(HistoryGraphError::InvalidRawPathHex)?;
            if String::from_utf8_lossy(&decoded).as_ref() != path.display {
                return Err(HistoryGraphError::InvalidRawPathHex);
            }
            decoded
        }
        None => path.display.as_bytes().to_vec(),
    };
    validate_path_bytes(&bytes, &path.display)?;
    Ok(bytes)
}

fn history_path_from_git(path: &GitPath) -> Result<HistoryPath, HistoryGraphError> {
    let bytes = git_path_bytes(path)?;
    let display = display_path_from_bytes(&bytes);
    let raw_bytes_hex = (display.as_bytes() != bytes.as_slice()).then(|| encode_hex(&bytes));
    let output = HistoryPath {
        display,
        raw_bytes_hex,
    };
    output.validate()?;
    Ok(output)
}

fn history_path_from_bytes(bytes: &[u8]) -> Result<HistoryPath, HistoryGraphError> {
    let display = display_path_from_bytes(bytes);
    let raw_bytes_hex = (display.as_bytes() != bytes).then(|| encode_hex(bytes));
    let path = HistoryPath {
        display,
        raw_bytes_hex,
    };
    path.validate()?;
    Ok(path)
}

fn history_change_kind(kind: &GitChangeKind) -> HistoryChangeKind {
    match kind {
        GitChangeKind::Added => HistoryChangeKind::Added,
        GitChangeKind::Modified => HistoryChangeKind::Modified,
        GitChangeKind::Deleted => HistoryChangeKind::Deleted,
        GitChangeKind::Renamed => HistoryChangeKind::Renamed,
        GitChangeKind::Copied => HistoryChangeKind::Copied,
        GitChangeKind::TypeChanged => HistoryChangeKind::TypeChanged,
        GitChangeKind::Unmerged => HistoryChangeKind::Unmerged,
        GitChangeKind::Other(value) => HistoryChangeKind::Other(value.clone()),
    }
}

fn change_kind_identity(kind: &GitChangeKind) -> String {
    match kind {
        GitChangeKind::Added => "added".to_owned(),
        GitChangeKind::Modified => "modified".to_owned(),
        GitChangeKind::Deleted => "deleted".to_owned(),
        GitChangeKind::Renamed => "renamed".to_owned(),
        GitChangeKind::Copied => "copied".to_owned(),
        GitChangeKind::TypeChanged => "type_changed".to_owned(),
        GitChangeKind::Unmerged => "unmerged".to_owned(),
        GitChangeKind::Other(value) => format!("other:{value}"),
    }
}

fn normalized_parent_ids(commit: &GitCommit) -> Result<Vec<String>, HistoryGraphError> {
    commit
        .parent_commit_ids
        .iter()
        .map(|parent| normalize_commit_id(parent))
        .collect()
}

fn normalize_commit_id(value: &str) -> Result<String, HistoryGraphError> {
    if !matches!(value.len(), 40 | 64) || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HistoryGraphError::InvalidCommitId(value.to_owned()));
    }
    Ok(value.to_ascii_lowercase())
}

#[allow(clippy::too_many_arguments)]
fn add_relationship(
    relationships: &mut BTreeMap<String, HistoryRelationship>,
    id_generator: &mut RandomIdGenerator,
    semantic_relationships: &mut HashSet<(
        HistoryRelationshipKind,
        String,
        String,
        Option<HistoryPathRole>,
    )>,
    context: &HistoryGraphContext,
    kind: HistoryRelationshipKind,
    from: &str,
    to: &str,
    path_role: Option<HistoryPathRole>,
) -> Result<(), HistoryGraphError> {
    let semantic_key = (
        kind.clone(),
        from.to_owned(),
        to.to_owned(),
        path_role.clone(),
    );
    if !semantic_relationships.insert(semantic_key) {
        return Ok(());
    }
    let id = id_generator.next("history-relationship", relationship_kind_identity(&kind));
    let relationship = HistoryRelationship {
        id: id.clone(),
        kind,
        from: from.to_owned(),
        to: to.to_owned(),
        context: context.clone(),
        path_role,
    };
    relationships.insert(id, relationship);
    Ok(())
}

fn relationship_kind_identity(kind: &HistoryRelationshipKind) -> &'static str {
    match kind {
        HistoryRelationshipKind::ContainsCommit => "contains_commit",
        HistoryRelationshipKind::ParentCommit => "parent_commit",
        HistoryRelationshipKind::RecordsChange => "records_change",
        HistoryRelationshipKind::ChangesPath => "changes_path",
        HistoryRelationshipKind::ContainsPath => "contains_path",
    }
}

fn insert_entity(
    entities: &mut BTreeMap<String, HistoryEntity>,
    entity: HistoryEntity,
) -> Result<(), HistoryGraphError> {
    match entities.get(&entity.id) {
        Some(existing) if existing == &entity => Ok(()),
        Some(_) => Err(HistoryGraphError::DuplicateEntityId(entity.id)),
        None => {
            entities.insert(entity.id.clone(), entity);
            Ok(())
        }
    }
}

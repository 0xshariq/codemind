//! Deterministic conversion from raw Git facts to the repository-scoped
//! History Graph. This module never executes Git commands and never persists
//! graph data; the Git boundary supplies facts and a later storage phase owns
//! persistence/verification.

use crate::{
    git::{GitChangeKind, GitCommit, GitHistory, GitPath},
    source::identity::digest_bytes,
};
use std::collections::{BTreeMap, BTreeSet};

use super::model::{
    HISTORY_GRAPH_SCHEMA_VERSION, HistoryChangeKind, HistoryEntity, HistoryEntityData,
    HistoryGraph, HistoryGraphContext, HistoryGraphError, HistoryPath, HistoryPathRole,
    HistoryRelationship, HistoryRelationshipKind, decode_hex, display_path_from_bytes, encode_hex,
    validate_path_bytes,
};

/// Build a repository-scoped graph slice from raw `git/` history facts.
///
/// The graph identity includes workspace and repository identity, but not the
/// source snapshot ID or graph projection version. Re-indexing identical Git
/// history in a later source snapshot therefore keeps stable History Graph IDs.
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

    let mut entities: BTreeMap<String, HistoryEntity> = BTreeMap::new();
    let mut relationships: BTreeMap<String, HistoryRelationship> = BTreeMap::new();
    let repository_entity_id = entity_id("history-repository-", &context, "repository", &[]);
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

    // First add all scanned commit nodes, then boundary nodes for parent commits
    // referenced by the scan but outside the retained history window.
    for (commit_id, commit) in &commits_by_id {
        let parent_commit_ids = normalized_parent_ids(commit)?;
        let commit_entity_id = commit_entity_id(&context, commit_id);
        insert_entity(
            &mut entities,
            HistoryEntity {
                id: commit_entity_id,
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

    let mut boundary_commit_ids = BTreeSet::new();
    for commit in commits_by_id.values() {
        for parent_id in normalized_parent_ids(commit)? {
            if !commits_by_id.contains_key(&parent_id) {
                boundary_commit_ids.insert(parent_id);
            }
        }
    }
    for commit_id in &boundary_commit_ids {
        insert_entity(
            &mut entities,
            HistoryEntity {
                id: commit_entity_id(&context, commit_id),
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
            &context,
            HistoryRelationshipKind::ContainsCommit,
            &repository_entity_id,
            &commit_entity_id(&context, commit_id),
            None,
        )?;
    }

    for (commit_id, commit) in &commits_by_id {
        let from_id = commit_entity_id(&context, commit_id);
        for parent_id in normalized_parent_ids(commit)? {
            add_relationship(
                &mut relationships,
                &context,
                HistoryRelationshipKind::ParentCommit,
                &from_id,
                &commit_entity_id(&context, &parent_id),
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
            let change_kind_key = change_kind_identity(&change.kind);
            let old_presence = if old_path_bytes.is_some() {
                &b"present"[..]
            } else {
                &b"absent"[..]
            };
            let new_presence = if new_path_bytes.is_some() {
                &b"present"[..]
            } else {
                &b"absent"[..]
            };
            let change_identity_fields: Vec<&[u8]> = vec![
                commit_id.as_bytes(),
                change.status_code.as_bytes(),
                change_kind_key.as_bytes(),
                old_presence,
                old_path_bytes.as_deref().unwrap_or(&[]),
                new_presence,
                new_path_bytes.as_deref().unwrap_or(&[]),
            ];
            let change_entity_id = entity_id(
                "history-change-",
                &context,
                "change",
                &change_identity_fields,
            );
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
                    &context,
                    &repository_entity_id,
                    &path,
                    &path_bytes,
                )?;
                add_relationship(
                    &mut relationships,
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
                    &context,
                    &repository_entity_id,
                    &path,
                    &path_bytes,
                )?;
                add_relationship(
                    &mut relationships,
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
        // Boundary stubs prove at least this many parent commits are outside
        // the scanned window, even if the raw bounded scan reports only the
        // generic lower bound of one omitted commit.
        omitted_commits_at_least: history
            .omitted_commits_at_least
            .max(boundary_commit_ids.len() as u64),
        omitted_path_changes_at_least,
    };
    graph.validate()?;
    Ok(graph)
}

fn add_path_and_directories(
    entities: &mut BTreeMap<String, HistoryEntity>,
    relationships: &mut BTreeMap<String, HistoryRelationship>,
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
        let directory_id = path_entity_id(context, "history-directory-", "directory", &prefix);
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
            context,
            HistoryRelationshipKind::ContainsPath,
            &parent_entity_id,
            &directory_id,
            None,
        )?;
        parent_entity_id = directory_id;
    }

    // The provided path must be exactly the path represented by the raw bytes.
    let represented_path_bytes = path_bytes(file_path)?;
    if file_path_bytes != represented_path_bytes.as_slice() {
        return Err(HistoryGraphError::InvalidPath(file_path.display.clone()));
    }
    let file_id = path_entity_id(context, "history-file-", "file", file_path_bytes);
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

fn commit_entity_id(context: &HistoryGraphContext, commit_id: &str) -> String {
    entity_id(
        "history-commit-",
        context,
        "commit",
        &[commit_id.as_bytes()],
    )
}

fn path_entity_id(
    context: &HistoryGraphContext,
    prefix: &str,
    domain: &str,
    path_bytes: &[u8],
) -> String {
    entity_id(prefix, context, domain, &[path_bytes])
}

fn entity_id(
    prefix: &str,
    context: &HistoryGraphContext,
    domain: &str,
    identity_fields: &[&[u8]],
) -> String {
    let mut material = b"codemind-history-graph-entity-v1\0".to_vec();
    append_field(&mut material, context.workspace_id.as_bytes());
    append_field(&mut material, context.repository_id.as_bytes());
    append_field(&mut material, domain.as_bytes());
    for field in identity_fields {
        append_field(&mut material, field);
    }
    format!("{prefix}{}", digest_bytes(&material))
}

fn add_relationship(
    relationships: &mut BTreeMap<String, HistoryRelationship>,
    context: &HistoryGraphContext,
    kind: HistoryRelationshipKind,
    from: &str,
    to: &str,
    path_role: Option<HistoryPathRole>,
) -> Result<(), HistoryGraphError> {
    let kind_name = relationship_kind_identity(&kind);
    let role_name = match path_role.as_ref() {
        Some(HistoryPathRole::Old) => "old",
        Some(HistoryPathRole::New) => "new",
        None => "none",
    };
    let id = entity_id(
        "history-rel-",
        context,
        "relationship",
        &[
            kind_name.as_bytes(),
            from.as_bytes(),
            to.as_bytes(),
            role_name.as_bytes(),
        ],
    );
    let relationship = HistoryRelationship {
        id: id.clone(),
        kind,
        from: from.to_owned(),
        to: to.to_owned(),
        context: context.clone(),
        path_role,
    };
    match relationships.get(&id) {
        Some(existing) if existing == &relationship => Ok(()),
        Some(_) => Err(HistoryGraphError::DuplicateRelationshipId(id)),
        None => {
            relationships.insert(id, relationship);
            Ok(())
        }
    }
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

fn append_field(output: &mut Vec<u8>, field: &[u8]) {
    output.extend_from_slice(&(field.len() as u64).to_be_bytes());
    output.extend_from_slice(field);
}

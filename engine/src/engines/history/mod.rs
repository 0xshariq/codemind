//! Repository-scoped Git History Graph model and deterministic graph builder.
//!
//! `git/` reads commit objects and path changes. This module translates those
//! raw facts into language-agnostic CodeMind entities and relationships; it
//! deliberately does not run Git commands or persist to FalkorDB.

pub mod graph_builder;
pub mod model;

pub use graph_builder::build_history_graph;
pub use model::{
    HISTORY_GRAPH_SCHEMA_VERSION, HistoryChangeKind, HistoryEntity, HistoryEntityData,
    HistoryEntityKind, HistoryGraph, HistoryGraphContext, HistoryGraphError, HistoryGraphStats,
    HistoryPath, HistoryPathRole, HistoryRelationship, HistoryRelationshipKind,
};

#[cfg(test)]
mod level2_tests {
    use super::*;

    #[test]
    fn history_model_modules_compose_repository_and_commit_scope_without_cross_domain_imports() {
        let context = HistoryGraphContext::new("workspace", "repo").unwrap();
        let commit_id = "0123456789012345678901234567890123456789".to_owned();
        let repository = HistoryEntity {
            id: "repository".into(),
            context: context.clone(),
            data: HistoryEntityData::Repository {
                repository_id: "repo".into(),
            },
        };
        let commit = HistoryEntity {
            id: "commit".into(),
            context: context.clone(),
            data: HistoryEntityData::Commit {
                commit_id: commit_id.clone(),
                parent_commit_ids: Vec::new(),
                committed_at_unix_seconds: Some(0),
                author: Some("author".into()),
                subject: Some("initial".into()),
                metadata_complete: true,
                changed_paths_complete: Some(true),
                omitted_paths_at_least: 0,
                is_boundary_stub: false,
            },
        };
        let graph = HistoryGraph {
            schema_version: HISTORY_GRAPH_SCHEMA_VERSION,
            context: context.clone(),
            head_commit: commit_id,
            entities: vec![repository, commit],
            relationships: vec![HistoryRelationship {
                id: "contains".into(),
                kind: HistoryRelationshipKind::ContainsCommit,
                from: "repository".into(),
                to: "commit".into(),
                context,
                path_role: None,
            }],
            complete: true,
            omitted_commits_at_least: 0,
            omitted_path_changes_at_least: 0,
        };
        assert!(graph.validate().is_ok());
    }
}

#[cfg(test)]
mod level2_builder_tests {
    use super::{HistoryEntityData, HistoryGraphContext, build_history_graph};
    use crate::git::{GitChangeKind, GitCommit, GitHistory, GitPath, GitPathChange};

    #[test]
    fn graph_builder_composes_commit_ancestry_and_path_change_facts() {
        let head = "0123456789012345678901234567890123456789";
        let parent = "abcdefabcdefabcdefabcdefabcdefabcdefabcd";
        let history = GitHistory {
            head_commit: head.into(),
            commits: vec![GitCommit {
                commit_id: head.into(),
                parent_commit_ids: vec![parent.into()],
                committed_at_unix_seconds: 1_800_000_000,
                author: "CodeMind Test".into(),
                subject: "add module".into(),
                changed_paths: vec![GitPathChange {
                    kind: GitChangeKind::Added,
                    status_code: "A".into(),
                    old_path: None,
                    new_path: Some(GitPath {
                        display: "src/module.rs".into(),
                        raw_bytes_hex: None,
                    }),
                }],
                changed_paths_complete: true,
                omitted_paths_at_least: 0,
            }],
            complete: false,
            omitted_commits_at_least: 1,
        };

        let graph = build_history_graph(
            HistoryGraphContext::new("workspace-1", "repo-1").unwrap(),
            &history,
        )
        .unwrap();
        graph.validate().unwrap();

        let stats = graph.stats();
        assert_eq!(stats.repository_count, 1);
        assert_eq!(stats.commit_count, 2);
        assert_eq!(stats.boundary_commit_count, 1);
        assert_eq!(stats.omitted_commits_at_least, 1);
        assert!(!stats.complete);
        assert_eq!(stats.file_count, 1);
        assert_eq!(stats.change_count, 1);
        assert!(stats.relationship_count >= 4);
        assert!(graph.entities.iter().any(|entity| matches!(
            &entity.data,
            HistoryEntityData::File { path } if path.display == "src/module.rs"
        )));
        assert!(graph.entities.iter().any(|entity| matches!(
            &entity.data,
            HistoryEntityData::Commit {
                commit_id,
                is_boundary_stub: true,
                ..
            } if commit_id == parent
        )));
    }

    #[test]
    fn graph_builder_rejects_history_whose_head_is_not_in_the_scanned_commits() {
        let history = GitHistory {
            head_commit: "0123456789012345678901234567890123456789".into(),
            commits: vec![GitCommit {
                commit_id: "abcdefabcdefabcdefabcdefabcdefabcdefabcd".into(),
                parent_commit_ids: Vec::new(),
                committed_at_unix_seconds: 1_800_000_000,
                author: "CodeMind Test".into(),
                subject: "unrelated commit".into(),
                changed_paths: Vec::new(),
                changed_paths_complete: true,
                omitted_paths_at_least: 0,
            }],
            complete: true,
            omitted_commits_at_least: 0,
        };

        assert!(
            build_history_graph(
                HistoryGraphContext::new("workspace-1", "repo-1").unwrap(),
                &history,
            )
            .is_err()
        );
    }
}

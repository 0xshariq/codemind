pub mod handoffs;
pub mod model;
pub mod repository;
pub mod scope;
pub mod service;
pub mod validation;

pub use repository::MemoryRepository;

#[cfg(test)]
mod level2_tests {
    use super::{model::MemoryRecord, validation::validate_memory};

    fn record(scope: &str, owner_id: &str) -> MemoryRecord {
        serde_json::from_value(serde_json::json!({
            "memory_id": "memory-1",
            "owner_id": owner_id,
            "kind": "semantic",
            "scope": scope,
            "lifecycle": "candidate",
            "freshness": "fresh",
            "payload_json": "{}",
            "context": {
                "repository_id": "repo-1",
                "workspace_id": "workspace-1",
                "source_snapshot_id": "snapshot-1",
                "graph_version": "graph-1"
            },
            "evidence": [{
                "path": "src/lib.rs",
                "start_line": 1,
                "end_line": 1,
                "snapshot_id": "snapshot-1"
            }],
            "idempotency_key": "memory-1",
            "version": 1,
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }))
        .expect("same-folder memory model fixture should deserialize")
    }

    #[test]
    fn repository_memory_cannot_be_owned_by_another_repository() {
        assert!(validate_memory(&record("repository", "repo-other")).is_err());
        assert!(validate_memory(&record("repository", "")).is_ok());
        assert!(validate_memory(&record("repository", "repo-1")).is_ok());
    }
}

#[cfg(test)]
mod level2_scope_validation_tests {
    use super::{model::MemoryRecord, scope::AccessContext, validation::validate_memory};
    use crate::source::RepositoryContext;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo-1".into(),
            workspace_id: "workspace-1".into(),
            source_snapshot_id: "snapshot-1".into(),
            graph_version: "graph-1".into(),
        }
    }

    fn record(scope: &str, owner_id: &str) -> MemoryRecord {
        MemoryRecord {
            memory_id: "memory-2".into(),
            owner_id: owner_id.into(),
            kind: "semantic".into(),
            scope: scope.into(),
            lifecycle: "candidate".into(),
            freshness: "fresh".into(),
            payload_json: "{}".into(),
            context: context(),
            evidence: Vec::new(),
            idempotency_key: "memory-2".into(),
            version: 1,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn validated_memory_is_visible_only_to_matching_scope_owner_and_workspace() {
        let memory = record("user", "user-1");
        validate_memory(&memory).unwrap();

        let owner_access = AccessContext::new("user-1", "user", "workspace-1").unwrap();
        assert!(owner_access.allows_record(&memory.scope, &memory.owner_id, &memory.context,));

        let other_user = AccessContext::new("user-2", "user", "workspace-1").unwrap();
        assert!(!other_user.allows_record(&memory.scope, &memory.owner_id, &memory.context,));

        let wrong_scope = AccessContext::new("user-1", "team", "workspace-1").unwrap();
        assert!(!wrong_scope.allows_record(&memory.scope, &memory.owner_id, &memory.context,));

        let wrong_workspace = AccessContext::new("user-1", "user", "workspace-2").unwrap();
        assert!(!wrong_workspace.allows_record(&memory.scope, &memory.owner_id, &memory.context,));
    }

    #[test]
    fn repository_memory_accepts_unowned_or_matching_repository_owner_only() {
        let unowned = record("repository", "");
        validate_memory(&unowned).unwrap();
        let matching = record("repository", "repo-1");
        validate_memory(&matching).unwrap();
        let mismatched = record("repository", "repo-2");
        assert!(validate_memory(&mismatched).is_err());

        let access = AccessContext::new("user-1", "repository", "workspace-1").unwrap();
        assert!(access.allows_record("repository", "", &unowned.context));
        assert!(access.allows_record("repository", "repo-1", &matching.context));
        assert!(!access.allows_record("repository", "repo-2", &matching.context));
    }
}

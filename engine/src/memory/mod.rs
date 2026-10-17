pub mod handoffs;
pub mod model;
pub mod repository;
pub mod scope;
pub mod service;
pub mod validation;

pub use repository::MemoryRepository;

#[cfg(test)]
mod integration_tests {
    use super::scope::AccessContext;
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
    fn access_is_principal_and_workspace_bound() {
        let access = AccessContext::new("user-1", "user", "workspace").unwrap();
        assert!(access.allows_record("user", "user-1", &context()));
        assert!(!access.allows_record("user", "user-2", &context()));
        assert!(!access.allows_record(
            "user",
            "user-1",
            &RepositoryContext {
                workspace_id: "other".into(),
                ..context()
            }
        ));
    }

    #[test]
    fn handoff_transition_policy_is_single_and_terminal_safe() {
        assert!(super::handoffs::can_transition("pending", "claimed"));
        assert!(super::handoffs::can_transition("claimed", "consumed"));
        assert!(super::handoffs::can_transition("consumed", "consumed"));
        assert!(!super::handoffs::can_transition("consumed", "claimed"));
    }
}

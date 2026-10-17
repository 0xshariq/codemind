pub mod events;
pub mod operation;
pub mod progress;
pub mod recovery;

pub use operation::Operation;

// Level 2 lifecycle tests verify the operation state/event/recovery files as a
// single module. Persistence adapters must provide the durable backing.
#[cfg(test)]
mod integration_tests {
    use super::{
        operation::Operation,
        progress::Progress,
        recovery::{RecoverySnapshot, replay_from},
    };
    use crate::{contract::operation::OperationState, source::RepositoryContext};

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn real_operation_events_are_monotonic_and_replayable() {
        let mut operation = Operation::new("op", "request", "trace", context()).unwrap();
        let started = operation.begin_stage("coie").unwrap();
        let finished = operation.finish_stage("coie").unwrap();
        assert!(finished.metadata.sequence > started.metadata.sequence);
        assert_eq!(Progress::from_event(&started).sequence, 1);

        let events = vec![
            crate::contract::events::EngineEvent::Progress {
                metadata: started.metadata.clone(),
                engine: "coie".into(),
                stage: "coie".into(),
                message: "started".into(),
            },
            crate::contract::events::EngineEvent::Progress {
                metadata: finished.metadata.clone(),
                engine: "coie".into(),
                stage: "coie".into(),
                message: "completed".into(),
            },
        ];
        let snapshot = RecoverySnapshot { operation, events };
        assert_eq!(replay_from(&snapshot, 1).unwrap().len(), 1);
    }

    #[test]
    fn terminal_operation_cannot_reopen() {
        let mut operation = Operation::new("op", "request", "trace", context()).unwrap();
        operation.record.result_reference = Some("result".into());
        operation.transition(OperationState::Running).unwrap();
        operation.transition(OperationState::Completed).unwrap();
        assert!(operation.transition(OperationState::Running).is_err());
    }
}

use super::operation::Operation;
use crate::contract::events::EngineEvent;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoverySnapshot {
    pub operation: Operation,
    pub events: Vec<EngineEvent>,
}

impl RecoverySnapshot {
    pub fn validate(&self) -> Result<(), String> {
        self.operation
            .record
            .validate()
            .map_err(|error| error.to_string())?;

        let mut previous = 0;
        for event in &self.events {
            event.validate().map_err(|error| error.to_string())?;
            if event.metadata().operation_id != self.operation.record.operation_id {
                return Err("recovery event belongs to another operation".to_owned());
            }
            if event.metadata().sequence <= previous {
                return Err("recovery event sequence is not strictly increasing".to_owned());
            }
            previous = event.metadata().sequence;
        }

        if previous > self.operation.record.latest_event_sequence {
            return Err("recovery contains events newer than the operation record".to_owned());
        }
        Ok(())
    }
}

pub fn replay_from(
    snapshot: &RecoverySnapshot,
    after_sequence: u64,
) -> Result<Vec<EngineEvent>, String> {
    snapshot.validate()?;
    Ok(snapshot
        .events
        .iter()
        .filter(|event| event.metadata().sequence > after_sequence)
        .cloned()
        .collect())
}

pub async fn recover_from_storage<S: crate::graph_storage::GraphStorage>(
    storage: &S,
    context: &crate::source::RepositoryContext,
    operation_id: &str,
    after_sequence: u64,
    limit: usize,
) -> Result<RecoverySnapshot, crate::graph_storage::StorageError> {
    let record = storage.retrieve_operation(context, operation_id).await?;
    let events = storage
        .events_since(context, operation_id, after_sequence, limit)
        .await?;

    let snapshot = RecoverySnapshot {
        operation: Operation { record },
        events,
    };

    snapshot
        .validate()
        .map_err(crate::graph_storage::StorageError::Query)?;

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{contract::events::EngineEvent, source::RepositoryContext};

    #[test]
    fn recovery_accepts_a_terminal_event_persisted_with_its_record_sequence() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };

        let mut operation = Operation::new("op", "request", "trace", context).unwrap();

        operation.record.result_reference = Some("result".into());

        operation
            .transition(crate::contract::operation::OperationState::Running)
            .unwrap();

        operation
            .transition(crate::contract::operation::OperationState::Completed)
            .unwrap();

        let event = operation.completed_event().unwrap();

        let snapshot = RecoverySnapshot {
            operation,
            events: vec![event],
        };

        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn recovery_rejects_events_from_another_operation() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };

        let mut operation = Operation::new("op", "request", "trace", context.clone()).unwrap();

        let event = operation.begin_stage("coie").unwrap();

        let mut other = Operation::new("other", "request", "trace", context).unwrap();

        let other_event = other.begin_stage("coie").unwrap();

        let engine_event = EngineEvent::Progress {
            metadata: other_event.metadata,
            engine: "codemind".into(),
            stage: "coie".into(),
            message: "other".into(),
        };

        let snapshot = RecoverySnapshot {
            operation,
            events: vec![engine_event],
        };

        assert!(snapshot.validate().is_err());

        let _ = event;
    }
}

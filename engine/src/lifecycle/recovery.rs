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

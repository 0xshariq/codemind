use super::operation::Operation;
use crate::contract::operation::StageState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub sequence: u64,
    pub event_id: String,
    pub operation_id: String,
    pub request_id: String,
    pub trace_id: String,
    pub occurred_at: String,
    pub context: crate::source::RepositoryContext,
    pub engine: String,
    pub stage: String,
    pub stage_state: StageState,
    pub message: String,
}

impl Progress {
    pub fn from_event(event: &super::operation::OperationEvent) -> Self {
        Self {
            sequence: event.metadata.sequence,
            event_id: event.metadata.event_id.clone(),
            operation_id: event.metadata.operation_id.clone(),
            request_id: event.metadata.request_id.clone(),
            trace_id: event.metadata.trace_id.clone(),
            occurred_at: event.metadata.occurred_at.to_rfc3339(),
            context: event.metadata.context.clone(),
            engine: "codemind".to_owned(),
            stage: event.stage.clone().unwrap_or_else(|| "unknown".to_owned()),
            stage_state: event.stage_state.unwrap_or(match event.state {
                crate::contract::operation::OperationState::Running => StageState::Running,
                crate::contract::operation::OperationState::Completed => StageState::Completed,
                crate::contract::operation::OperationState::Failed => StageState::Failed,
                _ => StageState::Pending,
            }),
            message: event.message.clone().unwrap_or_default(),
        }
    }
}

/// Progress is now produced by actual operation events. This compatibility
/// helper describes the stage order only; it does not claim those stages ran.
pub fn investigation_stages(operation: &Operation) -> Vec<Progress> {
    let _ = operation;
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::{Progress, investigation_stages};
    use crate::lifecycle::operation::Operation;
    use crate::source::RepositoryContext;

    #[test]
    fn progress_is_derived_from_real_operation_events() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let mut operation = Operation::new("op", "request", "trace", context).unwrap();
        let event = operation.begin_stage("coie").unwrap();
        let progress = Progress::from_event(&event);
        assert_eq!(progress.operation_id, "op");
        assert!(investigation_stages(&operation).is_empty());
    }
}

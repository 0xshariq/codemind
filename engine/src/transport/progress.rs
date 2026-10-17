use crate::{contract::generated, lifecycle::progress::Progress};

pub fn to_proto(value: &Progress) -> generated::ProgressEvent {
    generated::ProgressEvent {
        event_id: value.event_id.clone(),
        sequence: value.sequence,
        engine: value.engine.clone(),
        stage: value.stage.clone(),
        timestamp: value.occurred_at.clone(),
        correlation: Some(generated::Correlation {
            request_id: value.request_id.clone(),
            trace_id: value.trace_id.clone(),
            operation_id: value.operation_id.clone(),
        }),
        context: Some(generated::RepositoryContext {
            repository_id: value.context.repository_id.clone(),
            workspace_id: value.context.workspace_id.clone(),
            source_snapshot_id: value.context.source_snapshot_id.clone(),
            graph_version: value.context.graph_version.clone(),
        }),
        message: value.message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::to_proto;
    use crate::{
        contract::operation::StageState, lifecycle::operation::Operation,
        lifecycle::progress::Progress, source::RepositoryContext,
    };

    #[test]
    fn progress_conversion_preserves_sequence_and_correlation() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let mut operation = Operation::new("op", "request", "trace", context).unwrap();
        let event = operation.begin_stage("coie").unwrap();
        let progress = Progress::from_event(&event);
        let proto = to_proto(&progress);
        assert_eq!(proto.sequence, 1);
        assert_eq!(proto.stage, "coie");
        assert_eq!(proto.correlation.unwrap().operation_id, "op");
        assert_eq!(progress.stage_state, StageState::Running);
    }
}

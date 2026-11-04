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
        stage_state: match value.stage_state {
            crate::contract::operation::StageState::Pending => "pending",
            crate::contract::operation::StageState::Running => "running",
            crate::contract::operation::StageState::Completed => "completed",
            crate::contract::operation::StageState::Skipped => "skipped",
            crate::contract::operation::StageState::Failed => "failed",
        }
        .to_owned(),
    }
}

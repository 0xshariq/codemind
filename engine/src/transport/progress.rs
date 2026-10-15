use crate::{
    contract::generated,
    lifecycle::{operation::Operation, progress::Progress},
};

pub fn to_proto(operation: &Operation, value: &Progress) -> generated::ProgressEvent {
    generated::ProgressEvent {
        event_id: format!("{}:{}", operation.operation_id, value.sequence),
        sequence: value.sequence,
        engine: value.engine.clone(),
        stage: value.stage.clone(),
        timestamp: String::new(),
        correlation: Some(generated::Correlation {
            request_id: String::new(),
            trace_id: String::new(),
            operation_id: operation.operation_id.clone(),
        }),
        context: Some(generated::RepositoryContext {
            repository_id: operation.context.repository_id.clone(),
            workspace_id: operation.context.workspace_id.clone(),
            source_snapshot_id: operation.context.source_snapshot_id.clone(),
            graph_version: operation.context.graph_version.clone(),
        }),
        message: value.message.clone(),
    }
}

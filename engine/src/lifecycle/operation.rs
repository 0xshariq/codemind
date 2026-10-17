use crate::{
    contract::{
        events::{EngineEvent, EventMetadata},
        operation::{OperationRecord, OperationState, StageState},
    },
    source::RepositoryContext,
};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub record: OperationRecord,
}

impl Operation {
    pub fn new(
        operation_id: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
        context: RepositoryContext,
    ) -> Result<Self, String> {
        let record = OperationRecord::new(operation_id, request_id, trace_id, context)
            .map_err(|error| error.to_string())?;
        Ok(Self { record })
    }

    pub fn transition(&mut self, next: OperationState) -> Result<(), String> {
        self.record
            .transition_to(next)
            .map_err(|error| error.to_string())
    }

    pub fn transition_at(
        &mut self,
        next: OperationState,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        self.record
            .transition_to_at(next, now)
            .map_err(|error| error.to_string())
    }

    pub fn begin_stage(&mut self, stage: impl Into<String>) -> Result<OperationEvent, String> {
        let stage = stage.into();
        self.record.current_stage = Some(stage.clone());
        if self.record.state == OperationState::Pending {
            self.transition(OperationState::Running)?;
        }
        self.event(stage, StageState::Running, "stage started")
    }

    pub fn finish_stage(&mut self, stage: impl Into<String>) -> Result<OperationEvent, String> {
        self.event(stage.into(), StageState::Completed, "stage completed")
    }

    pub fn graph_updated(&mut self) -> Result<EngineEvent, String> {
        let metadata = self.next_metadata()?;
        let event = EngineEvent::GraphUpdated {
            metadata,
            graph_version: self.record.context.graph_version.clone(),
        };
        event.validate().map_err(|error| error.to_string())?;
        self.record.latest_event_sequence = event.metadata().sequence;
        Ok(event)
    }

    pub fn completed_event(&mut self) -> Result<EngineEvent, String> {
        let metadata = self.next_metadata()?;
        let result_reference = self
            .record
            .result_reference
            .clone()
            .ok_or_else(|| "completed operation requires a result reference".to_owned())?;
        let event = EngineEvent::Completed {
            metadata,
            result_reference,
        };
        event.validate().map_err(|error| error.to_string())?;
        self.record.latest_event_sequence = event.metadata().sequence;
        Ok(event)
    }

    fn next_metadata(&self) -> Result<EventMetadata, String> {
        EventMetadata::new(
            self.record.operation_id.clone(),
            self.record.request_id.clone(),
            self.record.trace_id.clone(),
            self.record.context.clone(),
            self.record.latest_event_sequence + 1,
        )
        .map_err(|error| error.to_string())
    }

    pub fn event(
        &mut self,
        stage: String,
        stage_state: StageState,
        message: impl Into<String>,
    ) -> Result<OperationEvent, String> {
        let sequence = self.record.latest_event_sequence + 1;
        let metadata = EventMetadata::new(
            self.record.operation_id.clone(),
            self.record.request_id.clone(),
            self.record.trace_id.clone(),
            self.record.context.clone(),
            sequence,
        )
        .map_err(|error| error.to_string())?;
        let event = EngineEvent::Progress {
            metadata: metadata.clone(),
            engine: "codemind".to_owned(),
            stage: stage.clone(),
            message: message.into(),
        };
        event.validate().map_err(|error| error.to_string())?;
        let now = chrono::Utc::now();
        if now > self.record.updated_at {
            self.record.updated_at = now;
        }
        self.record.latest_event_sequence = sequence;
        self.record.current_stage = Some(stage.clone());
        Ok(OperationEvent {
            metadata,
            state: self.record.state,
            stage: Some(stage),
            stage_state: Some(stage_state),
            message: match event {
                EngineEvent::Progress { message, .. } => Some(message),
                _ => None,
            },
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationEvent {
    pub metadata: EventMetadata,
    pub state: OperationState,
    pub stage: Option<String>,
    pub stage_state: Option<StageState>,
    pub message: Option<String>,
}

impl OperationEvent {
    pub fn from_engine_event(event: &EngineEvent) -> Self {
        Self {
            metadata: event.metadata().clone(),
            state: match event {
                EngineEvent::Completed { .. } => OperationState::Completed,
                EngineEvent::Failed { .. } => OperationState::Failed,
                _ => OperationState::Running,
            },
            stage: match event {
                EngineEvent::Progress { stage, .. } => Some(stage.clone()),
                _ => None,
            },
            stage_state: None,
            message: match event {
                EngineEvent::Progress { message, .. } => Some(message.clone()),
                EngineEvent::Failed { message, .. } => Some(message.clone()),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Operation;
    use crate::contract::operation::OperationState;
    use crate::source::RepositoryContext;

    #[test]
    fn completed_event_requires_a_result_and_advances_sequence() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let mut operation = Operation::new("op", "request", "trace", context).unwrap();
        operation.begin_stage("coie").unwrap();
        assert!(operation.completed_event().is_err());
        operation.record.result_reference = Some("result".into());
        operation.transition(OperationState::Completed).unwrap();
        let event = operation.completed_event().unwrap();
        assert_eq!(
            event.metadata().sequence,
            operation.record.latest_event_sequence
        );
    }
}

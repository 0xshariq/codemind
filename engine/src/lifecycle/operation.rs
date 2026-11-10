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

    /// Build the terminal failure event after the operation has transitioned
    /// to `Failed`. The caller is responsible for setting `error_code` and
    /// persisting both the updated record and returned event through storage.
    pub fn failed_event(
        &mut self,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<EngineEvent, String> {
        if self.record.state != OperationState::Failed {
            return Err("failed event requires an operation in the Failed state".to_owned());
        }

        let code = code.into();
        if self.record.error_code.as_deref() != Some(code.as_str()) {
            return Err("failed event code must match the operation error code".to_owned());
        }

        let event = EngineEvent::Failed {
            metadata: self.next_metadata()?,
            code,
            message: message.into(),
        };
        event.validate().map_err(|error| error.to_string())?;

        self.record.latest_event_sequence = event.metadata().sequence;
        let now = Utc::now();
        if now > self.record.updated_at {
            self.record.updated_at = now;
        }
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
    use crate::contract::{
        context::RepositoryContext, events::EngineEvent, operation::OperationState,
    };

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repository".to_owned(),
            workspace_id: "workspace".to_owned(),
            source_snapshot_id: "snapshot".to_owned(),
            graph_version: "graph-v1".to_owned(),
        }
    }

    #[test]
    fn failed_event_is_terminal_and_advances_the_event_sequence() {
        let mut operation = Operation::new("operation", "request", "trace", context()).unwrap();
        let started = operation.begin_stage("repository-selection").unwrap();
        assert_eq!(started.metadata.sequence, 1);

        operation.record.error_code = Some("SELECTED_ENTITY_NOT_FOUND".to_owned());
        operation.transition(OperationState::Failed).unwrap();
        let failed = operation
            .failed_event(
                "SELECTED_ENTITY_NOT_FOUND",
                "selected symbol or file was not found",
            )
            .unwrap();

        assert!(matches!(
            &failed,
            EngineEvent::Failed { code, message, .. }
                if code == "SELECTED_ENTITY_NOT_FOUND"
                    && message == "selected symbol or file was not found"
        ));
        assert_eq!(failed.metadata().sequence, 2);
        assert_eq!(operation.record.latest_event_sequence, 2);
        assert_eq!(operation.record.state, OperationState::Failed);
        assert_eq!(
            operation.record.current_stage.as_deref(),
            Some("repository-selection")
        );
    }

    #[test]
    fn failed_event_rejects_non_failed_operations_and_mismatched_codes() {
        let mut operation = Operation::new("operation", "request", "trace", context()).unwrap();
        assert!(operation.failed_event("ERROR", "message").is_err());

        operation.begin_stage("repository-selection").unwrap();
        operation.record.error_code = Some("EXPECTED".to_owned());
        operation.transition(OperationState::Failed).unwrap();
        assert!(operation.failed_event("OTHER", "message").is_err());
    }
}

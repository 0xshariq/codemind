use super::{context::RepositoryContext, error::ContractError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_EVENT_TEXT_BYTES: usize = 4_096;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventMetadata {
    pub event_id: String,
    /// Monotonically increasing within one operation. Sequence allocation and
    /// replay de-duplication are responsibilities of the operation publisher.
    pub sequence: u64,
    pub operation_id: String,
    pub request_id: String,
    pub trace_id: String,
    pub context: RepositoryContext,
    pub occurred_at: DateTime<Utc>,
}

impl EventMetadata {
    pub fn new(
        operation_id: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
        context: RepositoryContext,
        sequence: u64,
    ) -> Result<Self, ContractError> {
        let metadata = Self {
            event_id: Uuid::new_v4().to_string(),
            sequence,
            operation_id: operation_id.into(),
            request_id: request_id.into(),
            trace_id: trace_id.into(),
            context,
            occurred_at: Utc::now(),
        };
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        validate_text("event_id", &self.event_id, 256)?;
        validate_text("operation_id", &self.operation_id, MAX_EVENT_TEXT_BYTES)?;
        validate_text("request_id", &self.request_id, MAX_EVENT_TEXT_BYTES)?;
        validate_text("trace_id", &self.trace_id, MAX_EVENT_TEXT_BYTES)?;
        self.context.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EngineEvent {
    Progress {
        metadata: EventMetadata,
        engine: String,
        stage: String,
        message: String,
    },
    GraphUpdated {
        metadata: EventMetadata,
        graph_version: String,
    },
    Completed {
        metadata: EventMetadata,
        result_reference: String,
    },
    Failed {
        metadata: EventMetadata,
        code: String,
        message: String,
    },
}

impl EngineEvent {
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            Self::Progress { metadata, .. }
            | Self::GraphUpdated { metadata, .. }
            | Self::Completed { metadata, .. }
            | Self::Failed { metadata, .. } => metadata,
        }
    }

    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Progress { .. } => "progress",
            Self::GraphUpdated { .. } => "graph_updated",
            Self::Completed { .. } => "completed",
            Self::Failed { .. } => "failed",
        }
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        self.metadata().validate()?;
        match self {
            Self::Progress {
                engine,
                stage,
                message,
                ..
            } => {
                validate_text("engine", engine, 128)?;
                validate_text("stage", stage, 128)?;
                validate_text("message", message, MAX_EVENT_TEXT_BYTES)?;
            }
            Self::GraphUpdated { graph_version, .. } => {
                // The payload can announce a new published graph version while
                // metadata retains the context under which the operation began.
                validate_text("graph_version", graph_version, MAX_EVENT_TEXT_BYTES)?;
            }
            Self::Completed {
                result_reference, ..
            } => {
                validate_text("result_reference", result_reference, MAX_EVENT_TEXT_BYTES)?;
            }
            Self::Failed { code, message, .. } => {
                validate_text("code", code, 128)?;
                validate_text("message", message, MAX_EVENT_TEXT_BYTES)?;
            }
        }
        Ok(())
    }
}

fn validate_text(field: &'static str, value: &str, max_bytes: usize) -> Result<(), ContractError> {
    if value.trim().is_empty() {
        return Err(ContractError::InvalidInput(format!("{field} is required")));
    }
    if value.len() > max_bytes {
        return Err(ContractError::LimitExceeded(format!(
            "{field} exceeds {max_bytes} bytes"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(ContractError::InvalidInput(format!(
            "{field} contains a control character"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{EngineEvent, EventMetadata};
    use crate::contract::context::RepositoryContext;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph-2".into(),
        }
    }

    #[test]
    fn event_requires_correlation_and_valid_context() {
        let metadata = EventMetadata::new("op", "request", "trace", context(), 1).unwrap();
        let event = EngineEvent::Progress {
            metadata,
            engine: "coie".into(),
            stage: "extract".into(),
            message: "Extraction started".into(),
        };
        assert_eq!(event.kind(), "progress");
        assert!(event.validate().is_ok());
    }

    #[test]
    fn graph_update_requires_a_published_version() {
        let metadata = EventMetadata::new("op", "request", "trace", context(), 2).unwrap();
        let event = EngineEvent::GraphUpdated {
            metadata: metadata.clone(),
            graph_version: "graph-2".into(),
        };
        assert!(event.validate().is_ok());
        let invalid = EngineEvent::GraphUpdated {
            metadata,
            graph_version: " ".into(),
        };
        assert!(invalid.validate().is_err());
    }
}

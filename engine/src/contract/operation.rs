use super::{context::RepositoryContext, error::ContractError, events::EventMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Pending,
    Running,
    Completed,
    Degraded,
    Failed,
    Cancelled,
}

impl OperationState {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StageState {
    Pending,
    Running,
    Completed,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationRecord {
    pub operation_id: String,
    pub request_id: String,
    pub trace_id: String,
    pub context: RepositoryContext,
    pub state: OperationState,
    pub current_stage: Option<String>,
    pub latest_event_sequence: u64,
    pub result_reference: Option<String>,
    pub error_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl OperationRecord {
    pub fn new(
        operation_id: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
        context: RepositoryContext,
    ) -> Result<Self, ContractError> {
        Self::new_at(operation_id, request_id, trace_id, context, Utc::now())
    }

    /// Deterministic constructor for persistence adapters and tests that need a
    /// controlled clock. It does not allocate a database or process-local ID.
    pub fn new_at(
        operation_id: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
        context: RepositoryContext,
        now: DateTime<Utc>,
    ) -> Result<Self, ContractError> {
        let record = Self {
            operation_id: operation_id.into(),
            request_id: request_id.into(),
            trace_id: trace_id.into(),
            context,
            state: OperationState::Pending,
            current_stage: None,
            latest_event_sequence: 0,
            result_reference: None,
            error_code: None,
            created_at: now,
            updated_at: now,
        };
        record.validate()?;
        Ok(record)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        for (field, value) in [
            ("operation_id", self.operation_id.as_str()),
            ("request_id", self.request_id.as_str()),
            ("trace_id", self.trace_id.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ContractError::InvalidInput(format!("{field} is required")));
            }
            if value.chars().any(char::is_control) || value.len() > 4_096 {
                return Err(ContractError::InvalidInput(format!(
                    "{field} is malformed or too long"
                )));
            }
        }
        self.context.validate()?;
        if self.updated_at < self.created_at {
            return Err(ContractError::InvalidInput(
                "operation updated_at precedes created_at".to_owned(),
            ));
        }
        if self
            .current_stage
            .as_deref()
            .is_some_and(|stage| stage.trim().is_empty() || stage.len() > 128)
        {
            return Err(ContractError::InvalidInput(
                "operation current_stage is empty or too long".to_owned(),
            ));
        }
        if self
            .result_reference
            .as_deref()
            .is_some_and(|reference| reference.trim().is_empty())
        {
            return Err(ContractError::InvalidInput(
                "operation result_reference cannot be empty when supplied".to_owned(),
            ));
        }
        if self
            .error_code
            .as_deref()
            .is_some_and(|code| code.trim().is_empty() || code.len() > 128)
        {
            return Err(ContractError::InvalidInput(
                "operation error_code is empty or too long".to_owned(),
            ));
        }
        if self.state == OperationState::Completed && self.result_reference.is_none() {
            return Err(ContractError::InvalidInput(
                "completed operation requires a persisted result reference".to_owned(),
            ));
        }
        if self.state == OperationState::Failed && self.error_code.is_none() {
            return Err(ContractError::InvalidInput(
                "failed operation requires a safe error code".to_owned(),
            ));
        }
        Ok(())
    }

    /// State transition policy shared by operation writers. Repeated delivery
    /// of the same state is idempotent; terminal operations cannot be reopened.
    pub fn can_transition(current: OperationState, next: OperationState) -> bool {
        if current == next {
            return true;
        }
        matches!(
            (current, next),
            (OperationState::Pending, OperationState::Running)
                | (OperationState::Pending, OperationState::Failed)
                | (OperationState::Pending, OperationState::Cancelled)
                | (OperationState::Running, OperationState::Completed)
                | (OperationState::Running, OperationState::Degraded)
                | (OperationState::Running, OperationState::Failed)
                | (OperationState::Running, OperationState::Cancelled)
                | (OperationState::Degraded, OperationState::Running)
                | (OperationState::Degraded, OperationState::Completed)
                | (OperationState::Degraded, OperationState::Failed)
                | (OperationState::Degraded, OperationState::Cancelled)
        )
    }

    pub fn transition_to(&mut self, next: OperationState) -> Result<(), ContractError> {
        self.transition_to_at(next, Utc::now())
    }

    pub fn transition_to_at(
        &mut self,
        next: OperationState,
        now: DateTime<Utc>,
    ) -> Result<(), ContractError> {
        if !Self::can_transition(self.state, next) {
            return Err(ContractError::Conflict(format!(
                "operation cannot transition from {:?} to {:?}",
                self.state, next
            )));
        }
        // Validate transition-dependent invariants before mutating the record,
        // so a rejected transition cannot leave an invalid in-memory state.
        if next == OperationState::Completed && self.result_reference.is_none() {
            return Err(ContractError::InvalidInput(
                "completed operation requires a persisted result reference".to_owned(),
            ));
        }
        if next == OperationState::Failed && self.error_code.is_none() {
            return Err(ContractError::InvalidInput(
                "failed operation requires a safe error code".to_owned(),
            ));
        }
        let mut candidate = self.clone();
        if candidate.state != next {
            candidate.state = next;
            // Persisted event time is monotonic even if the host clock moves back.
            if now > candidate.updated_at {
                candidate.updated_at = now;
            }
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationEvent {
    pub metadata: EventMetadata,
    pub state: OperationState,
    pub stage: Option<String>,
    pub stage_state: Option<StageState>,
    pub message: Option<String>,
}

impl OperationEvent {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.metadata.validate()?;
        if self.stage_state.is_some() && self.stage.as_deref().is_none_or(str::is_empty) {
            return Err(ContractError::InvalidInput(
                "stage_state requires a non-empty stage name".to_owned(),
            ));
        }
        if self
            .stage
            .as_deref()
            .is_some_and(|stage| stage.trim().is_empty() || stage.len() > 128)
        {
            return Err(ContractError::InvalidInput(
                "operation event stage is empty or too long".to_owned(),
            ));
        }
        if self
            .message
            .as_deref()
            .is_some_and(|message| message.len() > 4_096)
        {
            return Err(ContractError::LimitExceeded(
                "operation event message exceeds the event size bound".to_owned(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{OperationRecord, OperationState};
    use crate::contract::context::RepositoryContext;
    use chrono::{Duration, TimeZone, Utc};

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn operation_transitions_are_ordered_and_terminal_state_cannot_reopen() {
        let now = Utc.timestamp_opt(1_800_000_000, 0).unwrap();
        let mut record = OperationRecord::new_at("op", "req", "trace", context(), now).unwrap();
        assert!(
            record
                .transition_to_at(OperationState::Running, now + Duration::seconds(1))
                .is_ok()
        );
        record.result_reference = Some("result-1".into());
        assert!(
            record
                .transition_to_at(OperationState::Completed, now + Duration::seconds(2))
                .is_ok()
        );
        assert!(
            record
                .transition_to_at(OperationState::Failed, now + Duration::seconds(3))
                .is_err()
        );
        assert!(record.state.is_terminal());
    }

    #[test]
    fn same_state_delivery_is_idempotent_and_does_not_rewind_time() {
        let now = Utc.timestamp_opt(1_800_000_000, 0).unwrap();
        let mut record = OperationRecord::new_at("op", "req", "trace", context(), now).unwrap();
        record
            .transition_to_at(OperationState::Running, now + Duration::seconds(1))
            .unwrap();
        let updated_at = record.updated_at;
        record
            .transition_to_at(OperationState::Running, now)
            .unwrap();
        assert_eq!(record.updated_at, updated_at);
    }
}

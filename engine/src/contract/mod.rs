//! Internal Rust contracts for the CodeMind engine.
//!
//! Generated protobuf types are included exactly once here. The sibling
//! modules define validated domain-facing envelopes and do not duplicate the
//! generated wire schema.

pub mod context;
pub mod error;
pub mod events;
pub mod generated {
    tonic::include_proto!("codemind.v1");
}
pub mod intelligence;
pub mod memory;
pub mod operation;
pub mod repository;
pub mod result;

pub use context::RepositoryContext;

// Per the repository's testing convention, Level 2 tests for collaboration
// between files in this module live in this module-folder mod.rs. Focused
// single-contract behavior remains covered by each sibling file's own tests.

#[cfg(test)]
mod tests_intelligence {
    use super::result;

    use super::context::RepositoryContext;
    use super::intelligence::IntelligenceRequest;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn coie_and_tie_requests_are_bounded_and_typed() {
        use super::intelligence::{CoieQueryKind, CoieQueryRequest, TieQueryRequest};
        let context = context();
        let coie = CoieQueryRequest {
            context: context.clone(),
            kind: CoieQueryKind::QueryRelationships,
            query: "symbol-1".into(),
            limit: 20,
        };
        assert!(coie.validate().is_ok());
        let tie = TieQueryRequest {
            context,
            target_entity_ids: vec!["symbol-1".into(), "symbol-2".into()],
            limit: 10,
        };
        assert!(tie.validate().is_ok());
        let invalid_tie = TieQueryRequest {
            target_entity_ids: vec!["symbol-1".into(), "symbol-1".into()],
            ..tie
        };
        assert!(invalid_tie.validate().is_err());
    }

    #[test]
    fn request_rejects_unbounded_depth_and_results() {
        let mut request = IntelligenceRequest {
            context: context(),
            target_entity_id: "symbol-1".into(),
            max_depth: 1,
            max_results: 10,
        };
        assert!(request.validate().is_ok());
        request.max_depth = result::MAX_PATH_LENGTH + 1;
        assert!(request.validate().is_err());
        request.max_depth = 1;
        request.max_results = 0;
        assert!(request.validate().is_err());
    }
}

#[cfg(test)]
mod tests_result {
    use super::context::RepositoryContext;
    use super::result::{
        CanonicalResult, Completeness, Correlation, EvidenceReference, Freshness, Outcome,
        bounded_relationships, serialized_size,
    };

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    fn result(data: Vec<String>) -> CanonicalResult<Vec<String>> {
        CanonicalResult::new(
            context(),
            Correlation {
                request_id: "request".into(),
                trace_id: "trace".into(),
                operation_id: "operation".into(),
            },
            Outcome::Success,
            Completeness::Complete,
            Freshness::Current,
            data,
        )
    }

    #[test]
    fn validates_context_correlation_evidence_and_serialized_bound() {
        let mut value = result(vec!["symbol".into()]);
        value.evidence.push(EvidenceReference {
            path: "src/lib.rs".into(),
            line: 1,
            snapshot_id: "snapshot".into(),
        });
        assert!(value.validate().is_ok());
        assert!(serialized_size(&value).unwrap() > 0);

        value.evidence[0].line = 0;
        assert!(value.validate().is_err());
    }

    #[test]
    fn truncation_is_reported_as_partial_not_complete_success() {
        let value = bounded_relationships(result(vec!["a".into(), "b".into()]), 1);
        assert_eq!(value.outcome, Outcome::Partial);
        assert_eq!(value.completeness, Completeness::Truncated);
        assert_eq!(value.data.len(), 1);
        assert!(value.validate().is_ok());
    }

    #[test]
    fn empty_success_becomes_empty_outcome() {
        let value = bounded_relationships(result(Vec::<String>::new()), 5);
        assert_eq!(value.outcome, Outcome::Empty);
        assert_eq!(value.completeness, Completeness::Complete);
    }
}

#[cfg(test)]
mod tests_events {
    use super::context::RepositoryContext;
    use super::events::{EngineEvent, EventMetadata};

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

#[cfg(test)]
mod tests_operation {
    use super::context::RepositoryContext;
    use super::operation::{OperationRecord, OperationState};
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

#[cfg(test)]
mod level2_lifecycle_tests {
    use super::{
        context::RepositoryContext,
        events::{EngineEvent, EventMetadata},
        operation::{OperationRecord, OperationState},
        result::{CanonicalResult, Completeness, Correlation, Freshness, Outcome},
    };
    use chrono::{Duration, TimeZone, Utc};

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo-1".into(),
            workspace_id: "workspace-1".into(),
            source_snapshot_id: "snapshot-1".into(),
            graph_version: "graph-1".into(),
        }
    }

    #[test]
    fn operation_event_and_canonical_result_share_one_valid_correlation_contract() {
        let context = context();
        let correlation = Correlation {
            request_id: "request-1".into(),
            trace_id: "trace-1".into(),
            operation_id: "operation-1".into(),
        };
        let now = Utc.timestamp_opt(1_800_000_000, 0).unwrap();

        let mut operation = OperationRecord::new_at(
            correlation.operation_id.clone(),
            correlation.request_id.clone(),
            correlation.trace_id.clone(),
            context.clone(),
            now,
        )
        .unwrap();
        operation
            .transition_to_at(OperationState::Running, now + Duration::seconds(1))
            .unwrap();

        let metadata = EventMetadata::new(
            correlation.operation_id.clone(),
            correlation.request_id.clone(),
            correlation.trace_id.clone(),
            context.clone(),
            1,
        )
        .unwrap();
        let event = EngineEvent::Progress {
            metadata,
            engine: "coie".into(),
            stage: "query".into(),
            message: "Query started".into(),
        };
        event.validate().unwrap();
        assert_eq!(event.metadata().operation_id, operation.operation_id);
        assert_eq!(event.metadata().request_id, operation.request_id);
        assert_eq!(event.metadata().trace_id, operation.trace_id);
        assert_eq!(event.metadata().context, operation.context);

        let result = CanonicalResult::new(
            context.clone(),
            correlation.clone(),
            Outcome::Success,
            Completeness::Complete,
            Freshness::Current,
            vec!["symbol-1".to_owned()],
        );
        result.validate().unwrap();
        result
            .validate_context_contract(&operation.context)
            .unwrap();
        assert_eq!(result.correlation, correlation);

        operation.result_reference = Some("result-1".into());
        operation
            .transition_to_at(OperationState::Completed, now + Duration::seconds(2))
            .unwrap();
        operation.validate().unwrap();
        assert_eq!(operation.state, OperationState::Completed);
        assert_eq!(operation.result_reference.as_deref(), Some("result-1"));
    }
}

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
mod tests {
    use super::{
        context::RepositoryContext,
        events::{EngineEvent, EventMetadata},
        intelligence::{CoieResult, IntelligenceRequest},
        memory::MemoryWriteRequest,
        operation::{OperationEvent, OperationRecord, OperationState, StageState},
        repository::{RepositorySourceRequest, RepositorySourceResult},
        result::{CanonicalResult, Completeness, Correlation, Freshness, Outcome},
    };
    use crate::{
        engines::coie::model::CodeGraph,
        memory::model::{MAX_PAYLOAD_BYTES, MemoryRecord},
        source::{FileEntry, SourceManifest, SourceSnapshot},
    };
    use chrono::{Duration, Utc};
    use std::collections::BTreeMap;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo-1".into(),
            workspace_id: "workspace-1".into(),
            source_snapshot_id: "snapshot-1".into(),
            graph_version: "graph-1".into(),
        }
    }

    #[test]
    fn level2_operation_event_and_result_contracts_preserve_shared_context() {
        let context = context();
        let now = Utc::now();
        let mut operation =
            OperationRecord::new_at("operation-1", "request-1", "trace-1", context.clone(), now)
                .unwrap();
        operation
            .transition_to_at(OperationState::Running, now + Duration::milliseconds(1))
            .unwrap();

        let metadata = EventMetadata::new(
            operation.operation_id.clone(),
            operation.request_id.clone(),
            operation.trace_id.clone(),
            context.clone(),
            1,
        )
        .unwrap();
        let engine_event = EngineEvent::Progress {
            metadata: metadata.clone(),
            engine: "coie".into(),
            stage: "extract".into(),
            message: "Extracting supported source constructs".into(),
        };
        assert!(engine_event.validate().is_ok());

        let operation_event = OperationEvent {
            metadata,
            state: operation.state,
            stage: Some("extract".into()),
            stage_state: Some(StageState::Running),
            message: Some("Extraction started".into()),
        };
        assert!(operation_event.validate().is_ok());
        assert_eq!(operation_event.metadata.context, context);

        let result = CanonicalResult::new(
            operation.context.clone(),
            Correlation {
                request_id: operation.request_id.clone(),
                trace_id: operation.trace_id.clone(),
                operation_id: operation.operation_id.clone(),
            },
            Outcome::Success,
            Completeness::Complete,
            Freshness::Current,
            vec!["symbol-1".to_owned()],
        );
        assert!(result.validate().is_ok());
        assert!(result.validate_context(&operation.context).is_ok());
    }

    #[test]
    fn level2_intelligence_request_and_coie_result_enforce_the_same_snapshot() {
        let context = context();
        let request = IntelligenceRequest {
            context: context.clone(),
            target_entity_id: "symbol-1".into(),
            max_depth: 3,
            max_results: 20,
        };
        request.validate_for_context(&context).unwrap();

        let result = CoieResult {
            graph: CodeGraph {
                context: context.clone(),
                entities: Vec::new(),
                relationships: Vec::new(),
                diagnostics: Vec::new(),
            },
        };
        result.validate(&context).unwrap();
        let mut other_context = context;
        other_context.graph_version = "graph-2".into();
        assert!(result.validate(&other_context).is_err());
    }

    #[test]
    fn level2_source_capture_and_memory_write_contracts_remain_context_consistent() {
        let context = context();
        let snapshot = SourceSnapshot {
            root: ".".into(),
            context: context.clone(),
            manifest: SourceManifest {
                repository_id: context.repository_id.clone(),
                source_snapshot_id: context.source_snapshot_id.clone(),
                files: vec![FileEntry {
                    path: "src/lib.rs".into(),
                    language: Some(crate::source::Language::Rust),
                    bytes: 13,
                    digest: "digest-1".into(),
                }],
                omitted_files: Vec::new(),
            },
            files: BTreeMap::from([("src/lib.rs".into(), b"pub fn f() {}".to_vec())]),
        };
        assert_eq!(snapshot.file_text("src/lib.rs").unwrap(), "pub fn f() {}");
        let source_result = RepositorySourceResult::from(&snapshot);
        source_result.validate().unwrap();

        let source_request = RepositorySourceRequest {
            workspace_id: context.workspace_id.clone(),
            root: ".".into(),
            max_file_bytes: 1024,
        };
        source_request.validate().unwrap();

        let record = MemoryRecord {
            memory_id: "memory-1".into(),
            owner_id: "repository-owner".into(),
            kind: "semantic".into(),
            scope: "repository".into(),
            lifecycle: "candidate".into(),
            freshness: "fresh".into(),
            payload_json: "{\"summary\":\"review context\"}".into(),
            context: context.clone(),
            evidence: Vec::new(),
            idempotency_key: "memory-write-1".into(),
            version: 1,
            created_at: "2026-10-16T00:00:00Z".into(),
            updated_at: "2026-10-16T00:00:00Z".into(),
        };
        assert!(record.payload_json.len() <= MAX_PAYLOAD_BYTES);
        let request = MemoryWriteRequest {
            context,
            scope: "repository".into(),
            owner_id: "repository-owner".into(),
            record,
        };
        request.validate().unwrap();
    }
}

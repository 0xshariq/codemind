//! Shared support for the four opt-in public-GitHub Level 3 acceptance tests.
//!
//! This is a module included by the four test targets, not a fifth test target.

use crate::support;
use chrono::Utc;
use codemind_engine::{
    contract::{events::EngineEvent, operation::OperationState},
    engines::{
        coie::model::{EntityKind, GraphEntity},
        history::{HistoryGraphContext, model::HistoryEntityData},
        pipeline::{InvestigationPipeline, VerifiedPipelineRequest},
    },
    graph_storage::{
        GraphStorage, GraphVerificationExpectation, MemoryRepository, generation_id_for_context,
    },
    memory::model::{EvidenceReference, MemoryRecord},
    repository::{
        AnalysisState, CleanupState, KnowledgeCompleteness, RepositoryState, workspace_scope_key,
    },
    source::{Language, SourceManifest},
};
use serde_json::json;
use std::{path::Path, sync::Arc};

pub struct RemoteRepositoryCase {
    pub slug: &'static str,
    pub url: &'static str,
    pub expected_languages: &'static [Language],
    /// Some repositories intentionally contain static tests; the engine must
    /// discover these declarations without executing the repository's tests.
    pub minimum_static_test_nodes: u64,
    /// Require captured Jupyter notebooks to be visible as unsupported source
    /// analysis, making the repository completeness result Partial.
    pub require_ipynb_as_incomplete: bool,
}

/// Exercise a real public remote through repository preparation, Git clone,
/// source/History/TIE indexing, storage verification, cleanup, reuse and ChIE.
///
/// The repository is intentionally not built and none of its own tests are
/// executed. TIE is static source discovery only.
pub async fn run_remote_repository(case: RemoteRepositoryCase) {
    let run_id = support::unique_run_id();
    let workspace_id = format!("ultimate-workspace-{}-{run_id}", case.slug);
    let index_operation_id = format!("ultimate-index-{}-{run_id}", case.slug);
    let index_request_id = format!("ultimate-index-request-{}-{run_id}", case.slug);
    let index_trace_id = format!("ultimate-index-trace-{}-{run_id}", case.slug);

    let storage = Arc::new(support::connect_falkordb().await);
    let manager = support::repository_manager(Arc::clone(&storage));

    // Phase 1: the manager owns the remote clone and the entire initial
    // Code/History/Test indexing lifecycle.
    let indexed = manager
        .ensure_repository_ready(
            case.url,
            &workspace_id,
            None,
            &index_operation_id,
            &index_request_id,
            &index_trace_id,
        )
        .await
        .unwrap_or_else(|error| {
            panic!(
                "RepositoryManager failed to clone/index {}: {error}",
                case.url
            )
        });

    assert!(
        !indexed.reused,
        "a unique test workspace must be indexed initially"
    );
    assert_eq!(indexed.cleanup_state, CleanupState::Cleaned);
    assert_ne!(indexed.completeness, KnowledgeCompleteness::Unknown);
    assert!(matches!(indexed.head_commit.len(), 40 | 64));
    assert!(
        indexed
            .head_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    );
    assert!(!indexed.context.repository_id.is_empty());
    assert_eq!(indexed.context.workspace_id, workspace_id);
    assert!(!indexed.context.source_snapshot_id.is_empty());
    assert!(!indexed.context.graph_version.is_empty());

    // The operation workspace itself must be gone, not merely its source
    // directory. That proves the `.git` directory and workspace marker are
    // removed together with the checkout.
    let owned_workspace = manager
        .paths()
        .workspace_dir(&index_operation_id)
        .expect("valid operation workspace path");
    let owned_clone = manager
        .paths()
        .workspace_repository_dir(&index_operation_id)
        .expect("valid clone destination");
    assert!(
        !owned_workspace.exists(),
        "successful verified ingestion left the owned workspace behind: {}",
        owned_workspace.display()
    );
    assert!(
        !owned_clone.exists(),
        "successful verified ingestion left the cloned repository behind: {}",
        owned_clone.display()
    );

    // Inspect the exact source manifest produced from the remote clone. This
    // checks that each repository reaches the intended Tree-sitter adapter.
    let manifest_json = indexed
        .manifest_json
        .as_deref()
        .expect("a fresh remote clone must return its exact source manifest");
    let manifest: SourceManifest = serde_json::from_str(manifest_json)
        .expect("RepositoryManager must return a valid SourceManifest");
    manifest
        .validate()
        .expect("the captured source manifest must validate");
    assert_eq!(manifest.repository_id, indexed.context.repository_id);
    assert_eq!(
        manifest.source_snapshot_id,
        indexed.context.source_snapshot_id
    );
    assert!(
        manifest
            .files
            .iter()
            .all(|file| { !file.path.split('/').any(|component| component == ".git") }),
        "Git metadata must be consumed by the Git boundary, not captured as source files"
    );

    for expected_language in case.expected_languages {
        let count = manifest
            .files
            .iter()
            .filter(|file| file.language == Some(*expected_language))
            .count();
        assert!(
            count > 0,
            "{} did not capture any files classified as {:?}; captured languages: {:?}",
            case.slug,
            expected_language,
            manifest
                .files
                .iter()
                .filter_map(|file| file.language)
                .map(Language::stable_name)
                .collect::<Vec<_>>()
        );
    }

    let notebook_paths = manifest
        .files
        .iter()
        .map(|file| file.path.as_str())
        .filter(|path| {
            Path::new(path)
                .extension()
                .is_some_and(|ext| ext == "ipynb")
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if case.require_ipynb_as_incomplete {
        assert!(
            !notebook_paths.is_empty(),
            "{} must exercise at least one real Jupyter notebook",
            case.slug
        );
        assert_eq!(
            manifest.unsupported_source_file_count(),
            notebook_paths.len() as u64,
            "every captured notebook must be surfaced as an unsupported source-analysis input"
        );
        assert_eq!(
            indexed.completeness,
            KnowledgeCompleteness::Partial,
            "unsupported notebook files must prevent a Complete repository claim"
        );
        assert!(!indexed.complete);
    }

    // Independently reload the durable repository state and reconstruct the
    // exact expected read-after-write verification contract from its receipt.
    let scope_key = workspace_scope_key(&workspace_id).expect("valid workspace scope key");
    let state = RepositoryState::load(manager.paths(), &indexed.context.repository_id, &scope_key)
        .expect("the ready repository state must be durably readable");
    assert_eq!(state.analysis_state, AnalysisState::Ready);
    assert_eq!(state.cleanup_state, CleanupState::Cleaned);
    assert_eq!(state.workspace_id, workspace_id);
    assert_eq!(
        state.last_workspace_operation_id.as_deref(),
        Some(index_operation_id.as_str()),
        "the durable repository state must retain the ID of the owned clone operation"
    );
    assert_eq!(
        state.last_verified_commit.as_deref(),
        Some(indexed.head_commit.as_str())
    );
    assert_eq!(
        state.last_source_snapshot_id.as_deref(),
        Some(indexed.context.source_snapshot_id.as_str())
    );
    assert_eq!(
        state.graph_projection_version.as_deref(),
        Some(indexed.context.graph_version.as_str())
    );

    let receipt = state
        .verification
        .as_ref()
        .expect("Ready state must carry a graph verification receipt");
    assert!(receipt.graph_names.iter().any(|name| name == "code"));
    assert!(receipt.graph_names.iter().any(|name| name == "history"));
    assert!(receipt.graph_names.iter().any(|name| name == "test"));
    assert_eq!(receipt.verified_commit, indexed.head_commit);
    assert!(
        receipt.test_entities >= case.minimum_static_test_nodes,
        "{} persisted {} static TIE test nodes, expected at least {}",
        case.slug,
        receipt.test_entities,
        case.minimum_static_test_nodes
    );

    let generation_id = generation_id_for_context(&indexed.context);
    assert_eq!(receipt.generation_id, generation_id);
    assert!(
        storage
            .verified_snapshot_exists(&indexed.context)
            .await
            .expect("read repository snapshot verification"),
        "the source snapshot is not verified in the graph store"
    );
    assert!(
        storage
            .verify_published_generation(&indexed.context, &generation_id)
            .await
            .expect("read graph publication marker"),
        "the verified graph generation is not the published generation"
    );

    let expected = GraphVerificationExpectation {
        context: indexed.context.clone(),
        generation_id: generation_id.clone(),
        code_entities: receipt.code_entities as usize,
        code_relationships: receipt.code_relationships as usize,
        code_entity_ids_digest: receipt.code_entity_ids_digest.clone(),
        code_relationship_ids_digest: receipt.code_relationship_ids_digest.clone(),
        test_entities: receipt.test_entities as usize,
        test_relationships: receipt.test_relationships as usize,
        test_entity_ids_digest: receipt.test_entity_ids_digest.clone(),
        test_relationship_ids_digest: receipt.test_relationship_ids_digest.clone(),
        history_entities: receipt.history_entities as usize,
        history_relationships: receipt.history_relationships as usize,
        history_head_commit: receipt.history_head_commit.clone(),
        history_complete: receipt.history_complete,
        history_omitted_commits_at_least: receipt.history_omitted_commits_at_least,
        history_omitted_path_changes_at_least: receipt.history_omitted_path_changes_at_least,
        change_entities: 0,
        change_relationships: 0,
    };
    let verification = storage
        .verify_graph(&indexed.context, &expected)
        .await
        .expect("independent read-after-write verification must execute");
    assert!(
        verification.satisfies(&expected),
        "the graph-store read-back did not satisfy the persisted verification receipt: {verification:?}"
    );

    let index_operation = storage
        .retrieve_operation(&indexed.context, &index_operation_id)
        .await
        .expect("read completed remote indexing operation");
    assert_eq!(index_operation.state, OperationState::Completed);
    let index_events = storage
        .events_since(&indexed.context, &index_operation_id, 0, 1_000)
        .await
        .expect("read remote indexing operation events");
    let indexed_stages = index_events
        .iter()
        .filter_map(|event| match event {
            EngineEvent::Progress { stage, .. } => Some(stage.as_str()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    for required_stage in [
        "repository-index.coie",
        "repository-index.history",
        "repository-index.tie",
        "repository-index.verify",
    ] {
        assert!(
            indexed_stages.contains(required_stage),
            "the remote repository operation did not record the `{required_stage}` stage"
        );
    }
    assert!(
        index_events
            .iter()
            .any(|event| matches!(event, EngineEvent::Completed { .. }))
    );

    // Query the persisted Code and History domains, then choose an exact,
    // non-ambiguous Symbol for a real storage-backed ChIE investigation.
    let code_entities = storage
        .search_code_entities(&indexed.context, "src/", 5_000)
        .await
        .expect("read Code Graph entities");
    assert!(
        code_entities
            .iter()
            .any(|entity| entity.kind == EntityKind::Symbol),
        "{} produced no persisted symbols under src/",
        case.slug
    );

    let mut selected_symbol: Option<GraphEntity> = None;
    for candidate in code_entities
        .iter()
        .filter(|entity| entity.kind == EntityKind::Symbol)
    {
        let exact = storage
            .exact_code_entities_by_name(&indexed.context, &candidate.name)
            .await
            .expect("read exact Code Graph symbol candidates");
        if exact.len() == 1
            && exact[0].kind == EntityKind::Symbol
            && exact[0].id == candidate.id
            && exact[0].path == candidate.path
            && exact[0].line == candidate.line
        {
            selected_symbol = Some(exact[0].clone());
            break;
        }
    }
    let selected_symbol = selected_symbol.unwrap_or_else(|| {
        panic!(
            "{} had no unique persisted Symbol available for a ChIE test",
            case.slug
        )
    });
    assert!(!selected_symbol.id.is_empty());

    let history_context = HistoryGraphContext::from_source_context(&indexed.context)
        .expect("construct repository-scoped History Graph context");
    let history_entities = storage
        .history_entities_for(&history_context, 5_000)
        .await
        .expect("read persisted History Graph entities");
    assert!(
        !history_entities.is_empty(),
        "{} persisted no History Graph entities",
        case.slug
    );
    assert!(
        history_entities.iter().any(|entity| matches!(
            &entity.data,
            HistoryEntityData::Commit { commit_id, .. }
                if commit_id.eq_ignore_ascii_case(&indexed.head_commit)
        )),
        "{} History Graph does not contain its actual cloned HEAD commit {}",
        case.slug,
        indexed.head_commit
    );
    assert!(
        history_entities
            .iter()
            .any(|entity| matches!(&entity.data, HistoryEntityData::Change { .. })),
        "{} History Graph contains no Git path-change facts",
        case.slug
    );

    // Run a second operation against the same remote and same authorized
    // workspace. It must verify and reuse the graph rather than cloning again.
    let investigation_operation_id = format!("ultimate-investigate-{}-{run_id}", case.slug);
    let investigation_request_id = format!("ultimate-investigate-request-{}-{run_id}", case.slug);
    let investigation_trace_id = format!("ultimate-investigate-trace-{}-{run_id}", case.slug);
    let reused = manager
        .ensure_indexed(
            case.url,
            &workspace_id,
            None,
            &selected_symbol.name,
            &investigation_operation_id,
            &investigation_request_id,
            &investigation_trace_id,
        )
        .await
        .unwrap_or_else(|error| {
            panic!(
                "RepositoryManager failed to reuse {} for ChIE: {error}",
                case.url
            )
        });
    assert!(
        reused.reused,
        "unchanged {} was cloned/reindexed instead of reused",
        case.slug
    );
    assert_eq!(reused.context, indexed.context);
    assert_eq!(reused.selected_entity_id, selected_symbol.id);
    assert_eq!(reused.cleanup_state, CleanupState::Cleaned);
    let investigation_workspace = manager
        .paths()
        .workspace_dir(&investigation_operation_id)
        .expect("valid second operation workspace path");
    assert!(
        !investigation_workspace.exists(),
        "repository reuse unexpectedly created a second clone: {}",
        investigation_workspace.display()
    );

    // Confirm TIE can be queried for the selected Code entity. A repository
    // with no tests may correctly return an empty graph; receipt verification
    // above still proves the exact persisted Test projection (including zero).
    let selected_tests = storage
        .tests_for_context(
            &reused.context,
            std::slice::from_ref(&reused.selected_entity_id),
            500,
        )
        .await
        .expect("read Test Graph relationships for the selected symbol");
    assert!(
        !selected_tests.truncated,
        "selected-symbol TIE result was truncated"
    );

    // ChIE must consume the stored Code/Test facts and publish a separately
    // verified Change Graph. It must not re-open or re-analyse the deleted clone.
    let investigation = InvestigationPipeline::new(Arc::clone(&storage))
        .run_verified(VerifiedPipelineRequest {
            context: reused.context.clone(),
            selected_entity_id: reused.selected_entity_id.clone(),
            request_id: investigation_request_id.clone(),
            trace_id: investigation_trace_id.clone(),
            operation_id: investigation_operation_id.clone(),
            knowledge_completeness: reused.completeness,
            freshness: reused.freshness,
            omitted_file_count: reused.omitted_file_count,
            history_complete: reused.history_complete,
            history_omitted_commits_at_least: reused.history_omitted_commits_at_least,
            history_omitted_path_changes_at_least: reused.history_omitted_path_changes_at_least,
        })
        .await
        .unwrap_or_else(|error| panic!("storage-backed ChIE failed for {}: {error}", case.slug));

    assert_eq!(investigation.operation_id, investigation_operation_id);
    assert_eq!(investigation.analysis_run.context, indexed.context);
    assert_eq!(investigation.analysis_run.status, "completed");
    assert_eq!(
        investigation.analysis_run.change_set_id,
        investigation.change_set.id
    );

    let change_relationships = storage
        .derived_relationships_for_run(&indexed.context, &investigation.analysis_run.id, 5_000)
        .await
        .expect("read separately published Change Graph relationships");
    assert!(
        change_relationships
            .iter()
            .any(|edge| edge.kind == "ANALYZES"),
        "ChIE Change Graph is missing ANALYZES"
    );
    assert!(
        change_relationships
            .iter()
            .any(|edge| edge.kind == "PRODUCES"),
        "ChIE Change Graph is missing PRODUCES"
    );

    let completed_operation = storage
        .retrieve_operation(&indexed.context, &investigation_operation_id)
        .await
        .expect("read completed ChIE operation");
    assert_eq!(completed_operation.state, OperationState::Completed);
    assert!(completed_operation.latest_event_sequence > 0);
    let events = storage
        .events_since(&indexed.context, &investigation_operation_id, 0, 1_000)
        .await
        .expect("read durable ChIE operation events");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, EngineEvent::Completed { .. }))
    );
    assert!(
        events
            .windows(2)
            .all(|pair| { pair[0].metadata().sequence < pair[1].metadata().sequence })
    );

    // A fresh storage adapter must see every graph domain after the original
    // connection is dropped, proving that the result is durable in FalkorDB.
    drop(manager);
    drop(storage);
    let reconnected = support::connect_falkordb().await;
    assert!(
        reconnected
            .entity_for(&indexed.context, &selected_symbol.id)
            .await
            .expect("read Code Graph after reconnect")
            .is_some(),
        "Code Graph was not durable across adapter reconnection"
    );
    assert!(
        !reconnected
            .history_entities_for(&history_context, 5_000)
            .await
            .expect("read History Graph after reconnect")
            .is_empty(),
        "History Graph was not durable across adapter reconnection"
    );
    let reconnected_verification = reconnected
        .verify_graph(&indexed.context, &expected)
        .await
        .expect("independently re-verify Code/History/Test after adapter reconnection");
    assert!(
        reconnected_verification.satisfies(&expected),
        "fresh adapter could not prove the complete Code/History/Test projection: {reconnected_verification:?}"
    );
    let reconnected_tests = reconnected
        .tests_for_context(
            &indexed.context,
            std::slice::from_ref(&selected_symbol.id),
            500,
        )
        .await
        .expect("read Test Graph after reconnect");
    assert!(!reconnected_tests.truncated);
    assert!(
        !reconnected
            .derived_relationships_for_run(&indexed.context, &investigation.analysis_run.id, 5_000)
            .await
            .expect("read Change Graph after reconnect")
            .is_empty(),
        "Change Graph was not durable across adapter reconnection"
    );
    assert_eq!(
        reconnected
            .retrieve_operation(&indexed.context, &investigation_operation_id)
            .await
            .expect("read operation after reconnect")
            .state,
        OperationState::Completed
    );

    // Exercise the fifth logical domain, Agent Memory, in the very same
    // workspace graph. The evidence is pinned to the indexed snapshot and the
    // memory must remain isolated from a different workspace.
    let now = Utc::now().to_rfc3339();
    let memory_record = MemoryRecord {
        memory_id: format!("ultimate-memory-{}-{run_id}", case.slug),
        owner_id: indexed.context.repository_id.clone(),
        kind: "semantic".into(),
        scope: "repository".into(),
        lifecycle: "validated".into(),
        freshness: "fresh".into(),
        payload_json: serde_json::to_string(&json!({
            "summary": format!("Ultimate Level 3 indexed symbol {}", selected_symbol.name),
            "symbol_id": selected_symbol.id,
            "source_path": selected_symbol.path,
        }))
        .expect("memory payload must serialize"),
        context: indexed.context.clone(),
        evidence: vec![EvidenceReference {
            path: selected_symbol.path.clone(),
            start_line: selected_symbol.line,
            end_line: selected_symbol.line,
            snapshot_id: indexed.context.source_snapshot_id.clone(),
        }],
        idempotency_key: format!("ultimate-memory-key-{}-{run_id}", case.slug),
        version: 1,
        created_at: now.clone(),
        updated_at: now,
    };
    let persisted_memory = reconnected
        .persist_memory(&memory_record)
        .await
        .expect("persist Agent Memory in the workspace graph");
    assert_eq!(persisted_memory.memory_id, memory_record.memory_id);
    assert_eq!(persisted_memory.context, indexed.context);
    assert_eq!(persisted_memory.evidence, memory_record.evidence);

    let memory_reader = support::connect_falkordb().await;
    let retrieved_memory = memory_reader
        .retrieve_memory(
            &indexed.context,
            "repository",
            "semantic",
            &indexed.context.repository_id,
            100,
        )
        .await
        .expect("read Agent Memory after adapter reconnection");
    let matching_memory = retrieved_memory
        .iter()
        .find(|record| record.memory_id == memory_record.memory_id)
        .expect("persisted Agent Memory must be visible after reconnection");
    assert_eq!(matching_memory.evidence, memory_record.evidence);
    assert_eq!(matching_memory.payload_json, memory_record.payload_json);

    let mut other_workspace_context = indexed.context.clone();
    other_workspace_context.workspace_id = format!("{}-isolated", indexed.context.workspace_id);
    let isolated_memory = memory_reader
        .retrieve_memory(
            &other_workspace_context,
            "repository",
            "semantic",
            &other_workspace_context.repository_id,
            100,
        )
        .await
        .expect("read Agent Memory in an unrelated workspace scope");
    assert!(
        !isolated_memory
            .iter()
            .any(|record| record.memory_id == memory_record.memory_id),
        "Agent Memory crossed the workspace graph boundary"
    );

    let manifest_path = support::persist_run_manifest(
        &run_id,
        &json!({
            "scenario": case.slug,
            "repository_url": case.url,
            "physical_falkordb_graph": support::physical_graph_name(&workspace_id),
            "workspace_id": workspace_id,
            "repository_id": indexed.context.repository_id,
            "source_snapshot_id": indexed.context.source_snapshot_id,
            "graph_projection_version": indexed.context.graph_version,
            "head_commit": indexed.head_commit,
            "knowledge_completeness": format!("{:?}", indexed.completeness),
            "freshness": format!("{:?}", indexed.freshness),
            "captured_file_count": manifest.files.len(),
            "omitted_file_count": manifest.omitted_files.len() + manifest.unsupported_source_file_count() as usize,
            "unsupported_source_file_count": manifest.unsupported_source_file_count(),
            "language_counts": case.expected_languages.iter().map(|language| {
                (language.stable_name(), manifest.files.iter().filter(|file| file.language == Some(*language)).count())
            }).collect::<std::collections::BTreeMap<_, _>>(),
            "notebook_paths": notebook_paths,
            "code_entity_count": receipt.code_entities,
            "test_entity_count": receipt.test_entities,
            "history_entity_count": receipt.history_entities,
            "history_complete": receipt.history_complete,
            "change_relationship_count": change_relationships.len(),
            "agent_memory_id": memory_record.memory_id,
            "agent_memory_retrieved_after_reconnect": true,
            "agent_memory_isolated_by_workspace": true,
            "selected_symbol": selected_symbol.name,
            "selected_symbol_path": selected_symbol.path,
            "selected_entity_id": selected_symbol.id,
            "analysis_run_id": investigation.analysis_run.id,
            "index_operation_id": index_operation_id,
            "investigation_operation_id": investigation_operation_id,
            "reuse_confirmed": reused.reused,
            "cleanup_state": format!("{:?}", indexed.cleanup_state),
            "owned_workspace_absent_after_indexing": !owned_workspace.exists(),
            "second_workspace_absent_after_reuse": !investigation_workspace.exists(),
            "graph_generation": generation_id,
        }),
    );

    println!(
        "ULTIMATE LEVEL 3 PASSED: {} | commit={} | files={} | symbols={} | static_tests={} | history_entities={} | change_relationships={} | agent_memory={} | reuse={} | cleanup={} | manifest={}",
        case.slug,
        indexed.head_commit,
        manifest.files.len(),
        code_entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Symbol)
            .count(),
        receipt.test_entities,
        history_entities.len(),
        change_relationships.len(),
        matching_memory.memory_id,
        reused.reused,
        indexed.cleanup_state == CleanupState::Cleaned,
        manifest_path.display()
    );
}

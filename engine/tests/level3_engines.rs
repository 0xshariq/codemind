//! Direct whole-engine acceptance test using the existing CodeMind checkout.
//!
//! This test is the primary Level 3 white-box path: repository preparation,
//! CoIE/History/TIE indexing, graph verification/publication, storage-backed
//! ChIE investigation, operation lifecycle and fresh-adapter read-back.

mod support;

use codemind_engine::{
    contract::{events::EngineEvent, operation::OperationState},
    engines::history::HistoryGraphContext,
    graph_storage::{GraphStorage, generation_id_for_context},
    repository::workspace_scope_key,
};
use serde_json::json;

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn real_codemind_repository_runs_through_all_four_graph_domains() {
    let prepared = support::prepare_local_codemind("engine-e2e").await;
    let context = prepared.knowledge.context.clone();
    let target_id = prepared.knowledge.selected_entity_id.clone();
    let generation_id = generation_id_for_context(&context);

    assert!(!prepared.knowledge.reused);
    assert_eq!(context.workspace_id, prepared.workspace_id);
    assert!(!context.repository_id.is_empty());
    assert!(!context.source_snapshot_id.is_empty());
    assert!(!context.graph_version.is_empty());
    assert!(!target_id.is_empty());
    assert!(!prepared.selected_target.path.is_empty());
    assert!(prepared.selected_target.line > 0);
    assert!(matches!(prepared.knowledge.head_commit.len(), 40 | 64));

    assert!(
        prepared
            .storage
            .verified_snapshot_exists(&context)
            .await
            .unwrap(),
        "RepositoryManager returned without a verified repository snapshot"
    );
    assert!(
        prepared
            .storage
            .verify_published_generation(&context, &generation_id)
            .await
            .unwrap(),
        "the active graph generation was not published"
    );

    // Code Graph read-back must find the exact target that CoIE/TIE selected.
    let selected = prepared
        .storage
        .entity_for(&context, &target_id)
        .await
        .unwrap()
        .expect("selected Code entity must have been persisted");
    assert_eq!(selected.name, prepared.selected_target.name);
    assert_eq!(selected.path, prepared.selected_target.path);
    assert_eq!(selected.context, context);
    support::assert_random_sha256_id_contract(
        &selected.id,
        "coie-entity:",
        "Persisted CoIE entity",
    );

    // History Graph is repository-scoped, not snapshot-scoped.
    let history_context = HistoryGraphContext::from_source_context(&context).unwrap();
    let history_entities = prepared
        .storage
        .history_entities_for(&history_context, 2_000)
        .await
        .unwrap();
    assert!(
        !history_entities.is_empty(),
        "Git history indexing produced no persisted History entities"
    );
    support::assert_history_graph_identity_contract(&history_entities, &[]);

    // TIE must persist the actual test that targets the selected symbol.
    let stored_tests = prepared
        .storage
        .tests_for_context(&context, std::slice::from_ref(&target_id), 200)
        .await
        .unwrap();
    assert!(
        !stored_tests.tests.is_empty(),
        "no discovered CodeMind test was persisted for the selected target"
    );
    support::assert_tie_identity_contract(&stored_tests.tests, &stored_tests.relationships);
    let has_selected_target =
        support::has_resolved_target_edge(&stored_tests.relationships, &target_id);
    assert!(
        has_selected_target,
        "persisted TIE graph has no resolved TARGETS edge to CoIE entity `{}` (symbol `{}` at {}:{}); returned {} tests and {} relationships (truncated={}): {:?}",
        target_id,
        prepared.selected_target.name,
        prepared.selected_target.path,
        prepared.selected_target.line,
        stored_tests.tests.len(),
        stored_tests.relationships.len(),
        stored_tests.truncated,
        stored_tests
            .relationships
            .iter()
            .filter(|relationship| {
                relationship.kind
                    == codemind_engine::engines::tie::model::TestRelationshipKind::Targets
            })
            .take(12)
            .map(|relationship| (
                relationship.entity_id.as_deref(),
                relationship.target_symbol_name.as_deref(),
                relationship.resolution
            ))
            .collect::<Vec<_>>()
    );

    // ChIE consumes persisted Code/History/Test records, not freshly-built test
    // fixtures or a newly extracted in-memory graph.
    let result = support::run_verified_investigation(&prepared).await;
    assert_eq!(result.operation_id, prepared.operation_id);
    assert_eq!(result.analysis_run.context, context);
    assert_eq!(result.analysis_run.status, "completed");
    support::assert_random_sha256_id_contract(
        &result.change_set.id,
        "change-set:",
        "ChIE ChangeSet",
    );
    support::assert_random_sha256_id_contract(
        &result.analysis_run.id,
        "analysis-run:",
        "ChIE AnalysisRun",
    );
    support::assert_random_sha256_id_contract(
        &result.analysis.analysis_id,
        "chie-analysis:",
        "ChIE analysis",
    );
    for evidence in &result.analysis.evidence_records {
        support::assert_random_sha256_id_contract(&evidence.id, "chie-evidence:", "ChIE evidence");
    }
    assert!(!result.change_set.selected_entity_id.is_empty());

    let change_relationships = prepared
        .storage
        .derived_relationships_for_run(&context, &result.analysis_run.id, 2_000)
        .await
        .unwrap();
    assert!(
        change_relationships
            .iter()
            .any(|edge| edge.kind == "ANALYZES"),
        "Change Graph is missing the ANALYZES relationship"
    );
    assert!(
        change_relationships
            .iter()
            .any(|edge| edge.kind == "PRODUCES"),
        "Change Graph is missing the PRODUCES relationship"
    );

    let operation = prepared
        .storage
        .retrieve_operation(&context, &prepared.operation_id)
        .await
        .unwrap();
    assert_eq!(operation.state, OperationState::Completed);
    assert!(operation.latest_event_sequence > 0);
    let events = prepared
        .storage
        .events_since(&context, &prepared.operation_id, 0, 1_000)
        .await
        .unwrap();
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

    // A second request for an unchanged checkout must reuse the exact verified
    // context and entity, not rebuild or cross a workspace boundary.
    let reuse_id = format!("level3-reuse-{}", prepared.run_id);
    let reused = prepared
        .manager
        .ensure_repository_ready(
            prepared.root.to_str().unwrap(),
            &prepared.workspace_id,
            None,
            &reuse_id,
            &format!("{reuse_id}-request"),
            &format!("{reuse_id}-trace"),
        )
        .await
        .unwrap();
    assert!(
        reused.reused,
        "unchanged verified knowledge was unexpectedly rebuilt"
    );
    assert_eq!(reused.context, context);
    let reused_candidates = prepared
        .storage
        .exact_code_entities_by_name(&context, &prepared.selected_target.name)
        .await
        .unwrap();
    assert_eq!(reused_candidates.len(), 1);
    assert_eq!(reused_candidates[0].name, prepared.selected_target.name);
    assert_eq!(reused_candidates[0].path, prepared.selected_target.path);
    assert_eq!(
        prepared
            .storage
            .retrieve_operation(&context, &reuse_id)
            .await
            .unwrap()
            .state,
        OperationState::Completed,
        "repository-only reuse should complete its own lifecycle operation"
    );

    let scope_key = workspace_scope_key(&prepared.workspace_id).unwrap();
    let metadata_path = prepared
        .manager
        .paths()
        .repository_metadata_path(&context.repository_id)
        .unwrap();
    let state_path = prepared
        .manager
        .paths()
        .repository_state_path(&context.repository_id, &scope_key)
        .unwrap();
    assert!(
        metadata_path.is_file(),
        "repository metadata was not persisted under ~/.codemind"
    );
    assert!(
        state_path.is_file(),
        "verified workspace state was not persisted under ~/.codemind"
    );

    let graph_name = support::physical_graph_name(&prepared.workspace_id);
    let manifest_path = support::persist_run_manifest(
        &prepared.run_id,
        &json!({
            "run_id": prepared.run_id.clone(),
            "repository_root": prepared.root.display().to_string(),
            "repository_id": context.repository_id.clone(),
            "workspace_id": prepared.workspace_id.clone(),
            "physical_falkordb_graph": graph_name,
            "source_snapshot_id": context.source_snapshot_id.clone(),
            "graph_version": context.graph_version.clone(),
            "generation_id": generation_id.clone(),
            "head_commit": prepared.knowledge.head_commit.clone(),
            "selected_symbol": prepared.selected_target.name.clone(),
            "selected_entity_id": target_id.clone(),
            "logical_graphs": ["code", "history", "test", "change"],
            "history_entity_count": history_entities.len(),
            "test_entity_count": stored_tests.tests.len(),
            "test_relationship_count": stored_tests.relationships.len(),
            "change_relationship_count": change_relationships.len(),
            "analysis_run_id": result.analysis_run.id.clone(),
            "operation_id": prepared.operation_id.clone(),
            "reuse_operation_id": reuse_id.clone(),
            "reuse_confirmed": reused.reused,
            "operation_event_count": events.len(),
            "repository_metadata_path": metadata_path.display().to_string(),
            "repository_state_path": state_path.display().to_string(),
        }),
    );

    // A fresh adapter must observe the same four logical graph domains and the
    // completed operation after the original client has gone away.
    let reconnected = support::connect_falkordb().await;
    assert!(
        reconnected
            .entity_for(&context, &target_id)
            .await
            .unwrap()
            .is_some(),
        "Code Graph was not durable across adapter reconnection"
    );
    assert!(
        !reconnected
            .history_entities_for(&history_context, 2_000)
            .await
            .unwrap()
            .is_empty(),
        "History Graph was not durable across adapter reconnection"
    );
    assert!(
        !reconnected
            .tests_for_context(&context, std::slice::from_ref(&target_id), 200)
            .await
            .unwrap()
            .tests
            .is_empty(),
        "Test Graph was not durable across adapter reconnection"
    );
    assert!(
        !reconnected
            .derived_relationships_for_run(&context, &result.analysis_run.id, 2_000)
            .await
            .unwrap()
            .is_empty(),
        "Change Graph was not durable across adapter reconnection"
    );
    assert_eq!(
        reconnected
            .retrieve_operation(&context, &prepared.operation_id)
            .await
            .unwrap()
            .state,
        OperationState::Completed
    );

    println!("Level 3 E2E passed for {}", prepared.root.display());
    println!("FalkorDB graph: {graph_name}");
    println!("Workspace: {}", prepared.workspace_id);
    println!("Selected symbol: {}", prepared.selected_target.name);
    println!("Run manifest: {}", manifest_path.display());
}

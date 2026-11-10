//! White-box FalkorDB adapter test over the graph produced by the real engine.
//! No graph nodes or relationships are fabricated by this test.

mod support;

use codemind_engine::{
    engines::history::HistoryGraphContext,
    graph_storage::{GraphStorage, generation_id_for_context},
};
use serde_json::json;

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn adapter_reads_all_persisted_graph_domains_with_workspace_context() {
    let prepared = support::prepare_local_codemind("storage-readback").await;
    let investigation = support::run_verified_investigation(&prepared).await;
    let context = prepared.knowledge.context.clone();
    let target_id = prepared.knowledge.selected_entity_id.clone();
    let history_context = HistoryGraphContext::from_source_context(&context).unwrap();
    let generation_id = generation_id_for_context(&context);

    let exact = prepared
        .storage
        .exact_code_entities_by_name(&context, &prepared.selected_target.name)
        .await
        .unwrap();
    assert_eq!(
        exact.len(),
        1,
        "selected symbol name must resolve uniquely in persisted CoIE data"
    );
    assert_eq!(exact[0].name, prepared.selected_target.name);
    assert_eq!(exact[0].path, prepared.selected_target.path);
    support::assert_random_sha256_id_contract(
        &exact[0].id,
        "coie-entity:",
        "Persisted CoIE entity",
    );

    let code_entity = prepared
        .storage
        .entity_for(&context, &target_id)
        .await
        .unwrap();
    assert!(code_entity.is_some());
    // Query the relationship and dependency read paths even when a particular
    // symbol has no outgoing edge. The assertion is that the actual adapter
    // executes each bounded query successfully against the indexed graph.
    let _outgoing = prepared
        .storage
        .relationships_for(&context, &target_id, 128)
        .await
        .unwrap();
    let _incoming = prepared
        .storage
        .incoming_relationships_for(&context, &target_id, 128)
        .await
        .unwrap();
    let _dependency_incoming = prepared
        .storage
        .dependency_incoming_relationships_for(&context, &target_id, 128)
        .await
        .unwrap();
    let dependency_paths = prepared
        .storage
        .dependency_paths_for(&context, &target_id, 3, 100)
        .await
        .unwrap();
    assert!(dependency_paths.paths.len() <= 100);

    let history = prepared
        .storage
        .history_entities_for(&history_context, 5_000)
        .await
        .unwrap();
    assert!(
        !history.is_empty(),
        "History Graph read returned no real Git history entities"
    );
    support::assert_history_graph_identity_contract(&history, &[]);

    let tests = prepared
        .storage
        .tests_for_context(&context, std::slice::from_ref(&target_id), 500)
        .await
        .unwrap();
    assert!(
        !tests.tests.is_empty(),
        "TIE did not persist any tests for the selected target"
    );
    support::assert_tie_identity_contract(&tests.tests, &tests.relationships);
    let has_selected_target = support::has_resolved_target_edge(&tests.relationships, &target_id);
    assert!(
        has_selected_target,
        "persisted TIE graph has no resolved TARGETS edge to CoIE entity `{}` (symbol `{}` at {}:{}); returned {} tests and {} relationships (truncated={}): {:?}",
        target_id,
        prepared.selected_target.name,
        prepared.selected_target.path,
        prepared.selected_target.line,
        tests.tests.len(),
        tests.relationships.len(),
        tests.truncated,
        tests
            .relationships
            .iter()
            .filter(|edge| {
                edge.kind == codemind_engine::engines::tie::model::TestRelationshipKind::Targets
            })
            .take(12)
            .map(|edge| (
                edge.entity_id.as_deref(),
                edge.target_symbol_name.as_deref(),
                edge.resolution
            ))
            .collect::<Vec<_>>()
    );

    let change = prepared
        .storage
        .derived_relationships_for_run(&context, &investigation.analysis_run.id, 5_000)
        .await
        .unwrap();
    assert!(
        !change.is_empty(),
        "ChIE produced no persisted Change Graph relationships"
    );

    assert!(
        prepared
            .storage
            .verified_snapshot_exists(&context)
            .await
            .unwrap()
    );
    assert!(
        prepared
            .storage
            .verify_published_generation(&context, &generation_id)
            .await
            .unwrap()
    );

    let expected_graph_name = support::physical_graph_name(&prepared.workspace_id);
    let manifest_path = support::persist_run_manifest(
        &prepared.run_id,
        &json!({
            "run_id": prepared.run_id.clone(),
            "repository_root": prepared.root.display().to_string(),
            "repository_id": context.repository_id.clone(),
            "workspace_id": context.workspace_id.clone(),
            "physical_falkordb_graph": expected_graph_name.clone(),
            "source_snapshot_id": context.source_snapshot_id.clone(),
            "graph_version": context.graph_version.clone(),
            "generation_id": generation_id,
            "selected_entity_id": target_id,
            "selected_symbol": prepared.selected_target.name.clone(),
            "code_entity_readable": code_entity.is_some(),
            "history_entity_count": history.len(),
            "test_entity_count": tests.tests.len(),
            "test_relationship_count": tests.relationships.len(),
            "change_relationship_count": change.len(),
            "dependency_path_count": dependency_paths.paths.len(),
            "analysis_run_id": investigation.analysis_run.id.clone(),
            "operation_id": prepared.operation_id.clone(),
        }),
    );

    // A new adapter instance proves the records are in FalkorDB, not only in
    // the first adapter's process-local schema cache or client state.
    let fresh_storage = support::connect_falkordb().await;
    assert!(
        fresh_storage
            .entity_for(&context, &prepared.knowledge.selected_entity_id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        !fresh_storage
            .history_entities_for(&history_context, 5_000)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        !fresh_storage
            .tests_for_context(
                &context,
                std::slice::from_ref(&prepared.knowledge.selected_entity_id),
                500
            )
            .await
            .unwrap()
            .tests
            .is_empty()
    );
    assert!(
        !fresh_storage
            .derived_relationships_for_run(&context, &investigation.analysis_run.id, 5_000)
            .await
            .unwrap()
            .is_empty()
    );

    println!("FalkorDB physical graph: {expected_graph_name}");
    println!("Level 3 storage manifest: {}", manifest_path.display());
}

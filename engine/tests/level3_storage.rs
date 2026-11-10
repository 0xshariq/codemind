//! White-box FalkorDB adapter test over the graph produced by the real engine.
//! No graph nodes or relationships are fabricated by this test.

mod support;

use codemind_engine::{
    engines::{coie::CoieEngine, history::HistoryGraphContext},
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


#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn adapter_reconciles_reingested_code_relationships_and_empty_test_graphs() {
    let prepared = support::prepare_local_codemind("storage-reingest").await;
    let investigation = support::run_verified_investigation(&prepared).await;
    let saved_analysis_links = prepared
        .storage
        .derived_relationships_for_run(
            &prepared.knowledge.context,
            &investigation.analysis_run.id,
            5_000,
        )
        .await
        .unwrap();
    let saved_affected_entity_id = saved_analysis_links
        .iter()
        .find(|relationship| relationship.kind == "AFFECTS")
        .map(|relationship| relationship.to.clone());
    let saved_analysis_relationships: Vec<_> = saved_analysis_links
        .iter()
        .filter(|relationship| {
            matches!(relationship.kind.as_str(), "AFFECTS" | "RECOMMENDS")
        })
        .map(|relationship| {
            (
                relationship.kind.clone(),
                relationship.from.clone(),
                relationship.to.clone(),
            )
        })
        .collect();
    if let Some(entity_id) = &saved_affected_entity_id {
        assert!(
            prepared
                .storage
                .entity_for(&prepared.knowledge.context, entity_id)
                .await
                .unwrap()
                .is_some(),
            "the saved investigation's CodeEntity target must exist before re-ingestion"
        );
    }
    let prior_target_tests = prepared
        .storage
        .tests_for_context(
            &prepared.knowledge.context,
            std::slice::from_ref(&prepared.knowledge.selected_entity_id),
            500,
        )
        .await
        .unwrap();
    assert!(
        !prior_target_tests.tests.is_empty(),
        "the selected CodeEntity must have persisted TIE references before re-ingestion"
    );

    let snapshot = support::local_snapshot(&prepared.root, &prepared.workspace_id).await;
    let first_extraction = CoieEngine
        .extract(&snapshot)
        .expect("extract first real repository graph");
    let second_extraction = CoieEngine
        .extract(&snapshot)
        .expect("extract the same snapshot again");
    assert_eq!(first_extraction.context, prepared.knowledge.context);
    let first_extracted_ids: std::collections::HashSet<_> = first_extraction
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect();
    assert!(
        second_extraction
            .entities
            .iter()
            .all(|entity| !first_extracted_ids.contains(entity.id.as_str())),
        "separate CoIE extractions must have fresh transient IDs"
    );

    let mut outgoing_counts = std::collections::HashMap::<&str, usize>::new();
    for relationship in &first_extraction.relationships {
        *outgoing_counts.entry(&relationship.from).or_default() += 1;
    }
    let transient_source_id = outgoing_counts
        .iter()
        .find_map(|(source, count)| (*count >= 2 && *count < 1_000).then_some(*source))
        .expect("real CodeMind graph must contain a node with multiple outgoing relationships");
    let candidates = first_extraction
        .relationships
        .iter()
        .enumerate()
        .filter(|(_, relationship)| relationship.from == transient_source_id)
        .map(|(index, _)| index)
        .take(2)
        .collect::<Vec<_>>();
    let omitted_index = candidates[0];
    let newly_added_index = candidates[1];
    assert!(omitted_index < newly_added_index);

    let mut first_write = first_extraction.clone();
    first_write.relationships.remove(newly_added_index);
    let canonical_first = prepared
        .storage
        .write_code_graph(&first_write)
        .await
        .unwrap();

    let mut second_write = second_extraction.clone();
    second_write.relationships.remove(omitted_index);
    let canonical_second = prepared
        .storage
        .write_code_graph(&second_write)
        .await
        .unwrap();

    let first_canonical_ids: std::collections::HashSet<_> = canonical_first
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect();
    let second_canonical_ids: std::collections::HashSet<_> = canonical_second
        .entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect();
    assert_eq!(
        first_canonical_ids, second_canonical_ids,
        "re-ingesting the same snapshot must reuse persisted CodeEntity IDs"
    );
    let second_relationship_ids: std::collections::HashSet<_> = canonical_second
        .relationships
        .iter()
        .map(|relationship| relationship.id.as_str())
        .collect();
    let shared_relationship_count = canonical_first
        .relationships
        .iter()
        .filter(|relationship| second_relationship_ids.contains(relationship.id.as_str()))
        .count();
    assert_eq!(
        shared_relationship_count,
        canonical_first.relationships.len() - 1,
        "relationships retained by both ingestions must keep their persisted IDs"
    );

    let omitted_id = canonical_first.relationships[omitted_index].id.clone();
    let source_id = canonical_first.relationships[omitted_index].from.clone();
    let newly_added_id = canonical_second.relationships[newly_added_index - 1]
        .id
        .clone();
    let current = prepared
        .storage
        .relationships_for(&canonical_second.context, &source_id, 5_000)
        .await
        .unwrap();
    assert!(
        current.iter().all(|relationship| relationship.id != omitted_id),
        "a relationship omitted by the second ingestion must no longer be current"
    );
    assert!(
        current.iter().any(|relationship| relationship.id == newly_added_id),
        "a relationship introduced by the second ingestion must be current"
    );
    if let Some(entity_id) = &saved_affected_entity_id {
        assert!(
            prepared
                .storage
                .entity_for(&canonical_second.context, entity_id)
                .await
                .unwrap()
                .is_some(),
            "re-ingestion must retain CodeEntity records referenced by saved AFFECTS evidence"
        );
    }
    let saved_links_after = prepared
        .storage
        .derived_relationships_for_run(
            &canonical_second.context,
            &investigation.analysis_run.id,
            5_000,
        )
        .await
        .unwrap();
    for (kind, from, to) in &saved_analysis_relationships {
        assert!(
            saved_links_after.iter().any(|relationship| {
                &relationship.kind == kind
                    && &relationship.from == from
                    && &relationship.to == to
            }),
            "re-ingestion removed a saved {kind} relationship from {from} to {to}"
        );
    }
    let retained_target_tests = prepared
        .storage
        .tests_for_context(
            &canonical_second.context,
            std::slice::from_ref(&prepared.knowledge.selected_entity_id),
            500,
        )
        .await
        .unwrap();
    assert!(
        !retained_target_tests.tests.is_empty(),
        "re-ingestion must retain CodeEntity records referenced by existing TARGETS/DECLARED_BY links"
    );

    // Re-ingesting an empty TIE projection is authoritative: previously
    // persisted test declarations must be removed from the same context.
    prepared
        .storage
        .write_test_graph(&canonical_second.context, &[], &[])
        .await
        .unwrap();
    let tests_after_empty_write = prepared
        .storage
        .tests_for_context(
            &canonical_second.context,
            std::slice::from_ref(&prepared.knowledge.selected_entity_id),
            500,
        )
        .await
        .unwrap();
    assert!(
        tests_after_empty_write.tests.is_empty(),
        "an empty Test Graph re-ingestion must clear previously persisted tests"
    );
    assert!(
        tests_after_empty_write.relationships.is_empty(),
        "an empty Test Graph re-ingestion must clear previously persisted test relationships"
    );
}

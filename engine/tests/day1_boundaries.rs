//! Real CoIE-to-FalkorDB adapter integration against the local CodeMind
//! checkout. This replaces the former synthetic graph/temporary one-file tests.

mod support;

use codemind_engine::{
    engines::coie::{CoieEngine, model::EntityKind},
    graph_storage::GraphStorage,
};
use serde_json::json;

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn coie_reingestion_replaces_random_graph_records_without_duplicates() {
    let root = support::codemind_repository_root();
    let run_id = support::unique_run_id();
    let workspace_id = format!("level3-coie-adapter-{run_id}");
    let snapshot = support::local_snapshot(&root, &workspace_id).await;
    let target = support::unique_test_targeted_symbol(&snapshot);
    let extracted = CoieEngine
        .extract(&snapshot)
        .expect("CoIE must extract the real CodeMind source snapshot");
    support::assert_code_graph_identity_contract(&extracted);
    let storage = support::connect_falkordb().await;

    let first_report = CoieEngine
        .ingest(&snapshot, &storage)
        .await
        .expect("CoIE must persist the real CodeMind graph through FalkorDbStorage");
    assert_eq!(first_report.context, snapshot.context);
    assert_eq!(first_report.entity_count, extracted.entities.len());
    assert_eq!(
        first_report.relationship_count,
        extracted.relationships.len()
    );
    assert!(first_report.entity_count > 0);
    assert!(first_report.relationship_count > 0);

    let selected_candidates = storage
        .exact_code_entities_by_name(&snapshot.context, &target.name)
        .await
        .unwrap();
    let selected = selected_candidates
        .iter()
        .find(|entity| entity.path == target.path)
        .expect("the actual test-targeted CodeMind symbol must be persisted");
    support::assert_random_sha256_id_contract(
        &selected.id,
        "coie-entity:",
        "Persisted CoIE entity",
    );
    assert_eq!(
        selected_candidates
            .iter()
            .filter(|entity| { entity.kind == EntityKind::Symbol && entity.path == target.path })
            .count(),
        1
    );
    assert_eq!(selected.kind, EntityKind::Symbol);
    assert_eq!(selected.name, target.name);
    assert_eq!(selected.path, target.path);
    assert_eq!(selected.context, snapshot.context);

    let file_candidates = storage
        .exact_code_entities_by_name(&snapshot.context, &target.path)
        .await
        .unwrap();
    let file_entity = file_candidates
        .iter()
        .find(|entity| entity.kind == EntityKind::File && entity.path == target.path)
        .expect("the selected symbol's source file must be persisted in the Code Graph");
    let first_edges = storage
        .relationships_for(&snapshot.context, &file_entity.id, 10_000)
        .await
        .unwrap();
    assert!(
        first_edges
            .iter()
            .any(|edge| edge.kind
                == codemind_engine::engines::coie::model::RelationshipKind::Defines),
        "the persisted file must have its actual Defines relationship"
    );
    // Re-ingest the identical immutable snapshot. Graph-record IDs are newly
    // minted, while the storage adapter must replace the old projection rather
    // than accumulate duplicate records.
    let second_report = CoieEngine
        .ingest(&snapshot, &storage)
        .await
        .expect("re-ingesting an identical snapshot must remain valid");
    assert_eq!(second_report.entity_count, first_report.entity_count);
    assert_eq!(
        second_report.relationship_count,
        first_report.relationship_count
    );
    let second_selected_candidates = storage
        .exact_code_entities_by_name(&snapshot.context, &target.name)
        .await
        .unwrap();
    let second_selected = second_selected_candidates
        .iter()
        .find(|entity| entity.path == target.path)
        .expect("re-ingestion must leave one current selected symbol");
    support::assert_random_sha256_id_contract(
        &second_selected.id,
        "coie-entity:",
        "Re-ingested CoIE entity",
    );
    assert_eq!(second_selected_candidates.len(), 1);
    let second_file_candidates = storage
        .exact_code_entities_by_name(&snapshot.context, &target.path)
        .await
        .unwrap();
    let second_file = second_file_candidates
        .iter()
        .find(|entity| entity.kind == EntityKind::File && entity.path == target.path)
        .expect("re-ingestion must leave one current source file");
    assert_eq!(
        second_file_candidates
            .iter()
            .filter(|entity| entity.kind == EntityKind::File)
            .count(),
        1
    );
    let second_edges = storage
        .relationships_for(&snapshot.context, &second_file.id, 10_000)
        .await
        .unwrap();
    assert_eq!(second_edges.len(), first_edges.len());
    assert!(
        second_edges
            .iter()
            .any(|edge| edge.kind
                == codemind_engine::engines::coie::model::RelationshipKind::Defines)
    );
    assert!(
        storage
            .entity_for(&snapshot.context, &selected.id)
            .await
            .unwrap()
            .is_none(),
        "the previous projection's selected entity must be replaced"
    );
    assert!(
        storage
            .entity_for(&snapshot.context, &file_entity.id)
            .await
            .unwrap()
            .is_none(),
        "the previous projection's file entity must be replaced"
    );

    // Workspace identity selects the physical graph. The same repository and
    // snapshot IDs in another workspace must not reveal the persisted entity.
    let mut other_workspace = snapshot.context.clone();
    other_workspace.workspace_id = format!("{workspace_id}-isolated");
    assert!(
        storage
            .entity_for(&other_workspace, &second_selected.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        storage
            .relationships_for(&other_workspace, &second_file.id, 10_000)
            .await
            .unwrap()
            .is_empty()
    );

    let graph_name = support::physical_graph_name(&workspace_id);
    let manifest_path = support::persist_run_manifest(
        &run_id,
        &json!({
            "run_id": run_id.clone(),
            "repository_root": root.display().to_string(),
            "repository_id": snapshot.context.repository_id.clone(),
            "workspace_id": workspace_id.clone(),
            "physical_falkordb_graph": graph_name.clone(),
            "source_snapshot_id": snapshot.context.source_snapshot_id.clone(),
            "graph_version": snapshot.context.graph_version.clone(),
            "selected_symbol": target.name.clone(),
            "selected_entity_name": target.name.clone(),
            "code_entity_count": first_report.entity_count,
            "code_relationship_count": first_report.relationship_count,
            "file_relationship_count": first_edges.len(),
            "random_id_reingest_replaced_projection": true,
            "other_workspace_isolated": true,
        }),
    );

    let reconnected = support::connect_falkordb().await;
    assert!(
        reconnected
            .entity_for(&snapshot.context, &second_selected.id)
            .await
            .unwrap()
            .is_some()
    );
    let durable_edges = reconnected
        .relationships_for(&snapshot.context, &second_file.id, 10_000)
        .await
        .unwrap();
    assert_eq!(durable_edges.len(), second_edges.len());

    println!(
        "CoIE adapter integration persisted {} entities and {} relationships",
        first_report.entity_count, first_report.relationship_count
    );
    println!("FalkorDB physical graph: {graph_name}");
    println!("Run manifest: {}", manifest_path.display());
}

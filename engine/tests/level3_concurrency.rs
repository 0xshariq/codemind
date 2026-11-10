//! Level 3 concurrency and parallel-workspace tests use the production
//! RepositoryManager, OS scope locks and FalkorDB workspace routing.

mod support;

use codemind_engine::{engines::history::HistoryGraphContext, graph_storage::GraphStorage};
use serde_json::json;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn concurrent_initialization_of_one_workspace_is_serialized_and_reused() {
    let _scenario_guard = support::acquire_heavy_level3_scenario_lock().await;
    let root = support::codemind_repository_root();
    let run_id = support::unique_run_id();
    let workspace_id = format!("level3-same-workspace-{run_id}");
    let identity_snapshot = support::local_snapshot(&root, &workspace_id).await;
    support::assert_source_tie_identity_contract(&identity_snapshot);
    let root_text = root.to_str().unwrap().to_owned();
    let storage = Arc::new(support::connect_falkordb().await);
    let manager = support::repository_manager(Arc::clone(&storage));

    let manager_a = Arc::clone(&manager);
    let workspace_a = workspace_id.clone();
    let operation_a = format!("level3-concurrent-a-{run_id}");
    let operation_a_for_call = operation_a.clone();
    let first = tokio::spawn(async move {
        manager_a
            .ensure_repository_ready(
                &root_text,
                &workspace_a,
                None,
                &operation_a_for_call,
                &format!("{operation_a_for_call}-request"),
                &format!("{operation_a_for_call}-trace"),
            )
            .await
    });

    let manager_b = Arc::clone(&manager);
    let workspace_b = workspace_id.clone();
    let operation_b = format!("level3-concurrent-b-{run_id}");
    let root_text_b = root.to_str().unwrap().to_owned();
    let operation_b_for_call = operation_b.clone();
    let second = tokio::spawn(async move {
        manager_b
            .ensure_repository_ready(
                &root_text_b,
                &workspace_b,
                None,
                &operation_b_for_call,
                &format!("{operation_b_for_call}-request"),
                &format!("{operation_b_for_call}-trace"),
            )
            .await
    });

    let (first_result, second_result) = tokio::join!(first, second);
    let first_knowledge = first_result
        .expect("first concurrent repository task must not panic")
        .expect("first concurrent repository request must succeed");
    let second_knowledge = second_result
        .expect("second concurrent repository task must not panic")
        .expect("second concurrent repository request must succeed");

    assert_eq!(first_knowledge.context, second_knowledge.context);
    assert_ne!(
        first_knowledge.reused, second_knowledge.reused,
        "one caller must perform initialization and the serialized caller must reuse its verified generation"
    );
    assert!(
        storage
            .verified_snapshot_exists(&first_knowledge.context)
            .await
            .unwrap()
    );
    assert!(
        storage
            .verify_published_generation(
                &first_knowledge.context,
                &codemind_engine::graph_storage::generation_id_for_context(
                    &first_knowledge.context
                ),
            )
            .await
            .unwrap()
    );
    for operation_id in [&operation_a, &operation_b] {
        let operation = storage
            .retrieve_operation(&first_knowledge.context, operation_id)
            .await
            .unwrap();
        assert_eq!(
            operation.state,
            codemind_engine::contract::operation::OperationState::Completed,
            "each concurrent repository request must reach a terminal state"
        );
    }

    let manifest_path = support::persist_run_manifest(
        &run_id,
        &json!({
            "run_id": run_id.clone(),
            "workspace_id": workspace_id.clone(),
            "physical_falkordb_graph": support::physical_graph_name(&workspace_id),
            "operation_ids": [operation_a, operation_b],
            "first_request_reused": first_knowledge.reused,
            "second_request_reused": second_knowledge.reused,
            "one_initialization_one_reuse": true,
        }),
    );
    println!(
        "Same-workspace concurrency manifest: {}",
        manifest_path.display()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn distinct_workspaces_can_index_in_parallel_without_cross_workspace_reads() {
    let _scenario_guard = support::acquire_heavy_level3_scenario_lock().await;
    let root = support::codemind_repository_root();
    let run_id = support::unique_run_id();
    let workspace_a = format!("level3-parallel-a-{run_id}");
    let workspace_b = format!("level3-parallel-b-{run_id}");
    let snapshot_a = support::local_snapshot(&root, &workspace_a).await;
    let target = support::unique_test_targeted_symbol(&snapshot_a);
    let root_a = root.to_str().unwrap().to_owned();
    let root_b = root_a.clone();
    let storage = Arc::new(support::connect_falkordb().await);
    let manager = support::repository_manager(Arc::clone(&storage));

    let manager_a = Arc::clone(&manager);
    let workspace_a_call = workspace_a.clone();
    let op_a = format!("level3-parallel-op-a-{run_id}");
    let op_a_call = op_a.clone();
    let task_a = tokio::spawn(async move {
        manager_a
            .ensure_repository_ready(
                &root_a,
                &workspace_a_call,
                None,
                &op_a_call,
                &format!("{op_a_call}-request"),
                &format!("{op_a_call}-trace"),
            )
            .await
    });

    let manager_b = Arc::clone(&manager);
    let workspace_b_call = workspace_b.clone();
    let op_b = format!("level3-parallel-op-b-{run_id}");
    let op_b_call = op_b.clone();
    let task_b = tokio::spawn(async move {
        manager_b
            .ensure_repository_ready(
                &root_b,
                &workspace_b_call,
                None,
                &op_b_call,
                &format!("{op_b_call}-request"),
                &format!("{op_b_call}-trace"),
            )
            .await
    });

    let (result_a, result_b) = tokio::join!(task_a, task_b);
    let knowledge_a = result_a
        .expect("workspace A indexing task must not panic")
        .expect("workspace A indexing must succeed");
    let knowledge_b = result_b
        .expect("workspace B indexing task must not panic")
        .expect("workspace B indexing must succeed");
    assert!(!knowledge_a.reused);
    assert!(!knowledge_b.reused);
    assert_ne!(
        knowledge_a.context.workspace_id,
        knowledge_b.context.workspace_id
    );
    assert_eq!(
        knowledge_a.context.repository_id,
        knowledge_b.context.repository_id
    );
    assert_ne!(
        support::physical_graph_name(&workspace_a),
        support::physical_graph_name(&workspace_b),
        "different workspaces must map to different physical FalkorDB graphs"
    );

    let entities_a = storage
        .exact_code_entities_by_name(&knowledge_a.context, &target.name)
        .await
        .unwrap();
    let entities_b = storage
        .exact_code_entities_by_name(&knowledge_b.context, &target.name)
        .await
        .unwrap();
    assert!(
        !entities_a.is_empty(),
        "workspace A has no persisted Code entity"
    );
    assert!(
        !entities_b.is_empty(),
        "workspace B has no persisted Code entity"
    );

    // A third, unindexed workspace cannot read either graph's Code, History or
    // Test data, even with the same repository/snapshot identity fields.
    let mut isolated_context = knowledge_a.context.clone();
    isolated_context.workspace_id = format!("level3-unindexed-{run_id}");
    assert!(
        storage
            .entity_for(&isolated_context, &entities_a[0].id)
            .await
            .unwrap()
            .is_none()
    );
    let isolated_history = HistoryGraphContext::from_source_context(&isolated_context).unwrap();
    assert!(
        storage
            .history_entities_for(&isolated_history, 100)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        storage
            .tests_for_context(&isolated_context, &[entities_a[0].id.clone()], 100)
            .await
            .unwrap()
            .tests
            .is_empty()
    );

    let manifest_path = support::persist_run_manifest(
        &run_id,
        &json!({
            "run_id": run_id.clone(),
            "repository_id": knowledge_a.context.repository_id.clone(),
            "workspace_a": workspace_a.clone(),
            "workspace_b": workspace_b.clone(),
            "physical_graph_a": support::physical_graph_name(&workspace_a),
            "physical_graph_b": support::physical_graph_name(&workspace_b),
            "workspace_a_code_entities": entities_a.len(),
            "workspace_b_code_entities": entities_b.len(),
            "unindexed_workspace_isolated": true,
            "parallel_initialization_succeeded": true,
        }),
    );
    println!("Parallel-workspace manifest: {}", manifest_path.display());
}

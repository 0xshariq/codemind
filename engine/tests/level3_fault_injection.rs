//! Level 3 failure-path tests. These use real adapter/network and repository
//! behavior; no fake GraphStorage implementation or mocked engine is involved.

mod support;

use codemind_engine::source::identity::RandomIdGenerator;
use codemind_engine::{
    contract::operation::OperationState, graph_storage::GraphStorage,
    repository::RepositoryManagerError,
};
use std::sync::Arc;

#[tokio::test]
#[ignore = "explicit Level 3 fault-injection test; uses a deliberately closed local TCP port"]
async fn falkordb_adapter_reports_a_real_connection_failure() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener); // The selected port has no listener when the adapter connects.

    let url = format!("redis://{address}");
    let context = support::repository_context(
        "level3-fault-repository",
        &format!("level3-fault-{}", support::unique_run_id()),
        "level3-fault-snapshot",
        "level3-fault-graph",
    );

    match support::connect_falkordb_with(&url, "codemind_level3_fault").await {
        Err(error) => {
            assert!(
                !error.to_string().trim().is_empty(),
                "adapter connection failure should retain a diagnostic: {error:?}"
            );
        }
        Ok(storage) => {
            // Some client builders establish sockets lazily. In that case the
            // first genuine storage command must expose the injected outage.
            let result = storage.verified_snapshot_exists(&context).await;
            assert!(
                result.is_err(),
                "FalkorDbStorage reported a successful read against a closed TCP port"
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn verified_chie_failure_persists_a_terminal_failed_operation() {
    let prepared = support::prepare_local_codemind("chie-fault").await;
    // Use a syntactically valid random CoIE ID that was never published.
    // This exercises missing-entity handling without depending on a manual ID.
    let missing_entity_id =
        RandomIdGenerator::default().next("coie-entity", "missing-selected-target");
    let error = support::run_verified_for_entity(&prepared, &missing_entity_id)
        .await
        .expect_err("ChIE must reject a selected entity absent from the verified Code Graph");
    assert!(
        error
            .to_string()
            .contains("selected entity is absent from verified graph"),
        "unexpected ChIE failure: {error}"
    );

    let context = &prepared.knowledge.context;
    let operation = prepared
        .storage
        .retrieve_operation(context, &prepared.operation_id)
        .await
        .unwrap();
    assert_eq!(operation.state, OperationState::Failed);
    assert!(operation.error_code.is_some());

    let events = prepared
        .storage
        .events_since(context, &prepared.operation_id, 0, 1_000)
        .await
        .unwrap();
    assert!(events.iter().any(|event| {
        matches!(
            event,
            codemind_engine::contract::events::EngineEvent::Progress {
                stage,
                message,
                ..
            } if stage == "chie" && message.contains("selected entity is absent from verified graph")
        )
    }), "failed ChIE operation must persist an explanatory failure event");

    let manifest_path = support::persist_run_manifest(
        &prepared.run_id,
        &serde_json::json!({
            "run_id": prepared.run_id.clone(),
            "workspace_id": prepared.workspace_id.clone(),
            "physical_falkordb_graph": support::physical_graph_name(&prepared.workspace_id),
            "operation_id": prepared.operation_id.clone(),
            "injected_failure": "selected entity absent from verified Code Graph",
            "operation_state": "failed",
            "error_code": operation.error_code,
            "event_count": events.len(),
        }),
    );
    println!("ChIE fault manifest: {}", manifest_path.display());
}

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn missing_selected_symbol_must_not_leave_a_running_operation_or_damage_the_graph() {
    let root = support::codemind_repository_root();
    let run_id = support::unique_run_id();
    let workspace_id = format!("level3-missing-symbol-{run_id}");
    let identity_snapshot = support::local_snapshot(&root, &workspace_id).await;
    support::assert_source_tie_identity_contract(&identity_snapshot);
    let operation_id = format!("level3-missing-symbol-op-{run_id}");
    let storage = Arc::new(support::connect_falkordb().await);
    let manager = support::repository_manager(Arc::clone(&storage));
    let root_text = root.to_str().unwrap();
    let missing_name = format!("__codemind_level3_missing_symbol_{run_id}__");

    let error = manager
        .ensure_indexed(
            root_text,
            &workspace_id,
            None,
            &missing_name,
            &operation_id,
            &format!("{operation_id}-request"),
            &format!("{operation_id}-trace"),
        )
        .await
        .expect_err("a symbol absent from the actual CodeMind repository must be rejected");
    let error_message = error.to_string();
    assert!(
        matches!(&error, &RepositoryManagerError::SelectedEntityNotFound),
        "expected SelectedEntityNotFound, received {error_message}"
    );

    // The repository graph may still be fully valid even though the caller's
    // selected entity did not resolve. The failed request must not corrupt or
    // silently downgrade that verified repository state.
    let knowledge = manager
        .ensure_repository_ready(
            root_text,
            &workspace_id,
            None,
            &format!("{operation_id}-recovery"),
            &format!("{operation_id}-recovery-request"),
            &format!("{operation_id}-recovery-trace"),
        )
        .await
        .expect("repository-level readiness must remain available after selection failure");
    assert!(
        knowledge.reused,
        "the valid repository generation should be reusable after a bad selection"
    );
    assert!(
        storage
            .verified_snapshot_exists(&knowledge.context)
            .await
            .unwrap()
    );

    // Prove the real graph domains remain readable after the failed selection.
    let snapshot = support::local_snapshot(&root, &workspace_id).await;
    let valid_target = support::unique_test_targeted_symbol(&snapshot);
    let persisted_candidates = storage
        .exact_code_entities_by_name(&knowledge.context, &valid_target.name)
        .await
        .unwrap();
    let persisted_target = persisted_candidates
        .iter()
        .find(|entity| entity.path == valid_target.path)
        .expect("the selected semantic target must exist in the persisted Code Graph");
    let persisted_code_entity = storage
        .entity_for(&knowledge.context, &persisted_target.id)
        .await
        .unwrap()
        .expect("selected CodeMind entity must remain readable after the failed selection");
    support::assert_random_sha256_id_contract(
        &persisted_code_entity.id,
        "coie-entity:",
        "Persisted CoIE entity",
    );
    let history_context =
        codemind_engine::engines::history::HistoryGraphContext::from_source_context(
            &knowledge.context,
        )
        .unwrap();
    let history_entities = storage
        .history_entities_for(&history_context, 2_000)
        .await
        .unwrap();
    assert!(!history_entities.is_empty());
    support::assert_history_graph_identity_contract(&history_entities, &[]);
    let stored_tests = storage
        .tests_for_context(
            &knowledge.context,
            std::slice::from_ref(&persisted_target.id),
            500,
        )
        .await
        .unwrap();
    assert!(!stored_tests.tests.is_empty());
    support::assert_tie_identity_contract(&stored_tests.tests, &stored_tests.relationships);

    // The repository generation remains Ready, but the request that asked
    // for a nonexistent symbol must be durably terminalized as Failed.
    let operation = storage
        .retrieve_operation(&knowledge.context, &operation_id)
        .await
        .unwrap();
    assert_eq!(
        operation.error_code.as_deref(),
        Some("SELECTED_ENTITY_NOT_FOUND"),
        "missing-symbol failure must persist a stable error code"
    );
    assert_eq!(
        operation.current_stage.as_deref(),
        Some("repository-selection"),
        "the operation must identify the stage that rejected the selection"
    );
    let failure_events = storage
        .events_since(&knowledge.context, &operation_id, 0, 1_000)
        .await
        .unwrap();
    let failed_event = failure_events.iter().find(|event| {
        matches!(
            event,
            codemind_engine::contract::events::EngineEvent::Failed { code, message, .. }
                if code == "SELECTED_ENTITY_NOT_FOUND"
                    && message.contains("selected symbol or file was not found")
        )
    });
    assert!(
        failed_event.is_some(),
        "missing-symbol selection must persist a terminal failure event"
    );
    assert_eq!(
        failure_events.last().map(|event| event.metadata().sequence),
        Some(operation.latest_event_sequence),
        "the persisted failure event must be the operation's latest event"
    );
    let manifest_path = support::persist_run_manifest(
        &run_id,
        &serde_json::json!({
            "run_id": run_id.clone(),
            "workspace_id": workspace_id.clone(),
            "physical_falkordb_graph": support::physical_graph_name(&workspace_id),
            "missing_symbol": missing_name,
            "request_error": error_message,
            "repository_generation_remained_verified": true,
            "valid_code_entity_remained_readable": true,
            "history_graph_remained_readable": true,
            "test_graph_remained_readable": true,
            "operation_state_observed": format!("{:?}", operation.state),
            "operation_stage_observed": operation.current_stage.clone(),
            "operation_error_code_observed": operation.error_code.clone(),
            "expected_operation_state": "Failed",
        }),
    );

    assert_eq!(
        operation.state,
        OperationState::Failed,
        "failed symbol selection must leave a terminal Failed operation, not {:?}; current stage: {:?}",
        operation.state,
        operation.current_stage
    );

    // Exercise the verified-reuse path too. It must create and fail its own
    // operation record even though no new graph indexing is required.
    let reuse_operation_id = format!("{operation_id}-reuse");
    let reuse_error = manager
        .ensure_indexed(
            root_text,
            &workspace_id,
            None,
            &missing_name,
            &reuse_operation_id,
            &format!("{reuse_operation_id}-request"),
            &format!("{reuse_operation_id}-trace"),
        )
        .await
        .expect_err("verified reuse must still reject a nonexistent selected symbol");
    assert!(
        matches!(&reuse_error, RepositoryManagerError::SelectedEntityNotFound),
        "verified-reuse selection returned an unexpected error: {reuse_error}"
    );
    let reuse_operation = storage
        .retrieve_operation(&knowledge.context, &reuse_operation_id)
        .await
        .unwrap();
    assert_eq!(reuse_operation.state, OperationState::Failed);
    assert_eq!(
        reuse_operation.error_code.as_deref(),
        Some("SELECTED_ENTITY_NOT_FOUND")
    );
    assert_eq!(
        reuse_operation.current_stage.as_deref(),
        Some("repository-selection")
    );
    let reuse_events = storage
        .events_since(&knowledge.context, &reuse_operation_id, 0, 1_000)
        .await
        .unwrap();
    assert!(reuse_events.iter().any(|event| matches!(
        event,
        codemind_engine::contract::events::EngineEvent::Failed { code, .. }
            if code == "SELECTED_ENTITY_NOT_FOUND"
    )));

    println!("Fault-injection manifest: {}", manifest_path.display());
}

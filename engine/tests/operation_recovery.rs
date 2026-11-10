//! Level 3 durable operation recovery over an actual verified CodeMind
//! investigation, not a manually assembled operation/event fixture.

mod support;

use codemind_engine::{
    contract::{events::EngineEvent, operation::OperationState},
    graph_storage::GraphStorage,
    lifecycle::recovery::{recover_from_storage, replay_from},
};
use serde_json::json;

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn completed_real_investigation_can_be_recovered_and_replayed_after_reconnect() {
    let prepared = support::prepare_local_codemind("operation-recovery").await;
    let investigation = support::run_verified_investigation(&prepared).await;
    let context = prepared.knowledge.context.clone();

    let first_connection_events = prepared
        .storage
        .events_since(&context, &prepared.operation_id, 0, 1_000)
        .await
        .unwrap();
    assert!(!first_connection_events.is_empty());
    assert!(
        first_connection_events
            .iter()
            .any(|event| matches!(event, EngineEvent::Completed { .. }))
    );

    // Recreate the adapter and recover solely from durable storage.
    let recovered_storage = support::connect_falkordb().await;
    let recovered = recover_from_storage(
        &recovered_storage,
        &context,
        &prepared.operation_id,
        0,
        1_000,
    )
    .await
    .unwrap();
    assert_eq!(
        recovered.operation.record.operation_id,
        prepared.operation_id
    );
    assert_eq!(recovered.operation.record.state, OperationState::Completed);
    assert_eq!(recovered.operation.record.context, context);
    assert_eq!(
        recovered.events, first_connection_events,
        "a new adapter must replay the exact durable event history"
    );
    assert!(recovered.validate().is_ok());
    assert!(
        recovered
            .events
            .iter()
            .any(|event| matches!(event, EngineEvent::Completed { .. }))
    );
    assert!(
        recovered
            .events
            .windows(2)
            .all(|pair| { pair[0].metadata().sequence < pair[1].metadata().sequence })
    );

    let replayed_all = replay_from(&recovered, 0).unwrap();
    assert_eq!(replayed_all, recovered.events);
    if let Some(first) = recovered.events.first() {
        let after_first = replay_from(&recovered, first.metadata().sequence).unwrap();
        assert!(
            after_first
                .iter()
                .all(|event| { event.metadata().sequence > first.metadata().sequence })
        );
    }

    let manifest_path = support::persist_run_manifest(
        &prepared.run_id,
        &json!({
            "run_id": prepared.run_id.clone(),
            "workspace_id": context.workspace_id.clone(),
            "physical_falkordb_graph": support::physical_graph_name(&context.workspace_id),
            "operation_id": prepared.operation_id.clone(),
            "analysis_run_id": investigation.analysis_run.id,
            "recovered_state": "completed",
            "durable_event_count": recovered.events.len(),
            "latest_event_sequence": recovered.operation.record.latest_event_sequence,
            "replay_count": replayed_all.len(),
        }),
    );
    println!("Operation recovery manifest: {}", manifest_path.display());
}

use codemind_engine::{
    contract::events::EngineEvent,
    contract::operation::OperationState,
    graph_storage::GraphStorage,
    lifecycle::{
        operation::Operation,
        recovery::{RecoverySnapshot, recover_from_storage, replay_from},
    },
    source::RepositoryContext,
};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires FalkorDB; run with --ignored"]
async fn persisted_operation_can_be_recovered_and_replayed_after_disconnect() {
    let graph_name = format!("codemind_level3_recovery_{}", Uuid::new_v4().simple());
    let storage = {
        let url = std::env::var("FALKORDB_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".into());
        codemind_engine::graph_storage::FalkorDbStorage::connect(&url, &graph_name)
            .await
            .unwrap()
    };
    let context = RepositoryContext {
        repository_id: "repo-recovery".into(),
        workspace_id: "workspace-recovery".into(),
        source_snapshot_id: "snapshot-recovery".into(),
        graph_version: "graph-recovery".into(),
    };
    let mut operation = Operation::new(
        format!("operation:{graph_name}"),
        "request",
        "trace",
        context.clone(),
    )
    .unwrap();
    let started = operation.begin_stage("coie").unwrap();
    storage.persist_operation(&operation.record).await.unwrap();
    let started_event = EngineEvent::Progress {
        metadata: started.metadata.clone(),
        engine: "codemind".into(),
        stage: "coie".into(),
        message: "started".into(),
    };
    storage
        .append_operation_event(&started_event)
        .await
        .unwrap();
    operation.record.result_reference = Some("result".into());
    operation.transition(OperationState::Completed).unwrap();
    storage.persist_operation(&operation.record).await.unwrap();

    let recovered =
        recover_from_storage(&storage, &context, &operation.record.operation_id, 0, 100)
            .await
            .unwrap();
    assert_eq!(
        recovered.operation.record.operation_id,
        operation.record.operation_id
    );
    assert_eq!(recovered.events.len(), 1);
    assert_eq!(
        recovered.events[0].metadata().sequence,
        started.metadata.sequence
    );
    let snapshot = RecoverySnapshot {
        operation: recovered.operation,
        events: recovered.events,
    };
    assert_eq!(replay_from(&snapshot, 0).unwrap().len(), 1);
}

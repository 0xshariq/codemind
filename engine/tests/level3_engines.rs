mod support;

use chrono::Utc;
use codemind_engine::{
    engines::chie::{AnalysisRun, ChangeSet, ChieEngine},
    graph_storage::GraphStorage,
};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires GitHub access and FalkorDB; run with --ignored"]
async fn coie_reads_codemind_from_github_and_builds_code_and_snapshot_history_provenance() {
    let Some((_root, snapshot, graph)) = support::github_code_graph() else {
        return;
    };

    support::assert_snapshot_provenance(&graph);

    assert!(
        !graph
            .entities
            .iter()
            .filter(|entity| {
                entity.kind == codemind_engine::engines::coie::model::EntityKind::File
            })
            .collect::<Vec<_>>()
            .is_empty()
    );

    assert!(graph.entities.iter().any(|entity| {
        entity.kind == codemind_engine::engines::coie::model::EntityKind::Symbol
    }));

    assert!(graph.relationships.iter().any(|relationship| {
        relationship.kind == codemind_engine::engines::coie::model::RelationshipKind::Defines
    }));

    assert!(graph.relationships.iter().any(|relationship| {
        relationship.kind == codemind_engine::engines::coie::model::RelationshipKind::Contains
    }));

    let graph_name = format!("codemind_level3_coie_{}", Uuid::new_v4().simple());
    let storage = support::storage(&graph_name).await;

    storage.write_code_graph(&graph).await.unwrap();

    let repository = graph
        .entities
        .iter()
        .find(|entity| entity.kind == codemind_engine::engines::coie::model::EntityKind::Repository)
        .unwrap();

    let source_snapshot = graph
        .entities
        .iter()
        .find(|entity| {
            entity.kind == codemind_engine::engines::coie::model::EntityKind::RepositorySnapshot
        })
        .unwrap();

    assert!(
        storage
            .entity_for(&snapshot.context, &repository.id)
            .await
            .unwrap()
            .is_some()
    );

    assert!(
        storage
            .entity_for(&snapshot.context, &source_snapshot.id)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
#[ignore = "requires GitHub access and FalkorDB; run with --ignored"]
async fn tie_reads_the_same_github_snapshot_and_builds_canonical_test_graph() {
    let Some((_root, snapshot, code_graph, tests, relationships)) = support::github_test_graph()
    else {
        return;
    };

    assert!(
        !tests.is_empty(),
        "CodeMind repository should contain discoverable tests"
    );

    assert!(relationships.iter().any(|relationship| {
        relationship.kind == codemind_engine::engines::tie::model::TestRelationshipKind::DeclaredBy
            && relationship.resolution
                == codemind_engine::engines::tie::model::TestResolution::Resolved
    }));

    assert!(relationships.iter().any(|relationship| {
        relationship.kind == codemind_engine::engines::tie::model::TestRelationshipKind::Targets
            && relationship.resolution
                == codemind_engine::engines::tie::model::TestResolution::Resolved
            && relationship.entity_id.is_some()
    }));

    let graph_name = format!("codemind_level3_tie_{}", Uuid::new_v4().simple());

    let storage = support::storage(&graph_name).await;

    storage.write_code_graph(&code_graph).await.unwrap();

    storage
        .write_test_graph(&snapshot.context, &tests, &relationships)
        .await
        .unwrap();

    let target_ids = support::test_target_ids(&relationships);

    let stored = storage
        .tests_for_context(&snapshot.context, &target_ids, 500)
        .await
        .unwrap();

    assert!(!stored.tests.is_empty());

    assert!(stored.relationships.iter().any(|relationship| {
        relationship.kind == codemind_engine::engines::tie::model::TestRelationshipKind::Targets
    }));
}

#[tokio::test]
#[ignore = "requires GitHub access and FalkorDB; run with --ignored"]
async fn chie_reads_code_and_test_graphs_from_the_github_snapshot_and_publishes_change_graph() {
    let Some((_root, snapshot, code_graph, tests, relationships)) = support::github_test_graph()
    else {
        return;
    };

    let selected = support::test_target_ids(&relationships)
        .into_iter()
        .next()
        .unwrap_or_else(|| support::first_callable_symbol(&code_graph));

    let analysis = support::analyze_with_chie(&code_graph, &tests, &relationships, &selected);

    assert_eq!(analysis.selected_entity_id, selected);
    assert!(!analysis.evidence_records.is_empty() || analysis.impact_paths.is_empty());

    let graph_name = format!("codemind_level3_chie_{}", Uuid::new_v4().simple());

    let storage = support::storage(&graph_name).await;

    storage.write_code_graph(&code_graph).await.unwrap();

    storage
        .write_test_graph(&snapshot.context, &tests, &relationships)
        .await
        .unwrap();

    let selected_entity = code_graph
        .entities
        .iter()
        .find(|entity| entity.id == selected)
        .unwrap();

    let change_set = ChangeSet::for_entity(
        format!("change-set:{graph_name}"),
        snapshot.context.clone(),
        selected.clone(),
        selected_entity.kind.clone(),
        selected_entity.evidence.clone(),
    )
    .unwrap();

    let change_record = ChieEngine::default()
        .change_record(&change_set.id, &code_graph, &selected)
        .unwrap();

    let analysis_run = AnalysisRun {
        id: format!("analysis-run:{graph_name}"),
        change_set_id: change_set.id.clone(),
        context: snapshot.context.clone(),
        status: "completed".into(),
        started_at: Utc::now().to_rfc3339(),
        completed_at: Some(Utc::now().to_rfc3339()),
    };

    storage
        .persist_change_analysis(&change_set, &change_record, &analysis_run, &analysis)
        .await
        .unwrap();

    let derived = storage
        .derived_relationships_for_run(&snapshot.context, &analysis_run.id, 200)
        .await
        .unwrap();

    assert!(
        derived
            .iter()
            .any(|relationship| relationship.kind == "PRODUCES")
    );

    if !analysis.impact_paths.is_empty() {
        assert!(
            derived
                .iter()
                .any(|relationship| relationship.kind == "AFFECTS")
        );
    }

    if !analysis.test_candidates.is_empty() {
        assert!(
            derived
                .iter()
                .any(|relationship| relationship.kind == "RECOMMENDS")
        );
    }

    assert!(
        derived
            .iter()
            .any(|relationship| relationship.kind == "ANALYZES")
    );
}

#[tokio::test]
#[ignore = "requires GitHub access and FalkorDB; run with --ignored"]
async fn pipeline_runs_the_complete_github_snapshot_path_and_leaves_replayable_operation_events() {
    let Some((_root, snapshot, graph)) = support::github_code_graph() else {
        return;
    };

    let selected = support::first_callable_symbol(&graph);

    let graph_name = format!("codemind_level3_pipeline_{}", Uuid::new_v4().simple());

    let storage = std::sync::Arc::new(support::storage(&graph_name).await);

    let pipeline = codemind_engine::engines::pipeline::InvestigationPipeline::new(
        std::sync::Arc::clone(&storage),
    );

    let request = codemind_engine::engines::pipeline::PipelineRequest {
        snapshot: snapshot.clone(),
        selected_entity_id: selected.clone(),
        request_id: format!("level3-request-{graph_name}"),
        trace_id: format!("level3-trace-{graph_name}"),
        operation_id: Some(format!("operation:{graph_name}")),
    };

    let result = pipeline.run(request).await.unwrap();

    assert_eq!(result.operation_id, format!("operation:{graph_name}"));

    assert!(!result.graph.entities.is_empty());

    assert!(result.graph.entities.iter().any(|entity| {
        entity.kind == codemind_engine::engines::coie::model::EntityKind::RepositorySnapshot
    }));

    assert!(!result.analysis_run.id.is_empty());

    let operation = storage
        .retrieve_operation(&snapshot.context, &result.operation_id)
        .await
        .unwrap();

    assert_eq!(
        operation.state,
        codemind_engine::contract::operation::OperationState::Completed
    );

    assert!(operation.latest_event_sequence > 0);

    let events = storage
        .events_since(&snapshot.context, &result.operation_id, 0, 200)
        .await
        .unwrap();

    assert!(!events.is_empty());

    assert!(
        events
            .windows(2)
            .all(|pair| { pair[0].metadata().sequence < pair[1].metadata().sequence })
    );

    assert!(events.iter().any(|event| matches!(
        event,
        codemind_engine::contract::events::EngineEvent::Progress { stage, message, .. }
            if stage == "coie" && message == "stage started"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        codemind_engine::contract::events::EngineEvent::Progress { stage, message, .. }
            if stage == "tie" && message == "stage started"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        codemind_engine::contract::events::EngineEvent::Progress { stage, message, .. }
            if stage == "chie" && message == "stage started"
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        codemind_engine::contract::events::EngineEvent::Completed { .. }
    )));

    let derived = storage
        .derived_relationships_for_run(&snapshot.context, &result.analysis_run.id, 200)
        .await
        .unwrap();

    assert!(
        derived
            .iter()
            .any(|relationship| relationship.kind == "ANALYZES")
    );

    assert!(
        derived
            .iter()
            .any(|relationship| relationship.kind == "PRODUCES")
    );
}

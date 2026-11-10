//! Bounded concurrent-read stress test over a real indexed and investigated
//! CodeMind workspace. All reads go through the production GraphStorage port.

mod support;

use codemind_engine::{engines::history::HistoryGraphContext, graph_storage::GraphStorage};
use futures::future::join_all;
use std::{sync::Arc, time::Duration};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires the local CodeMind Git checkout and a running FalkorDB instance"]
async fn concurrent_bounded_reads_remain_consistent_under_repeated_load() {
    let prepared = support::prepare_local_codemind("read-stress").await;
    let investigation = support::run_verified_investigation(&prepared).await;
    let context = prepared.knowledge.context.clone();
    let history_context = HistoryGraphContext::from_source_context(&context).unwrap();
    let target_id = prepared.knowledge.selected_entity_id.clone();
    let analysis_run_id = investigation.analysis_run.id.clone();
    let storage = Arc::clone(&prepared.storage);

    let worker_count = 8usize;
    let rounds_per_worker = 8usize;
    let mut workers = Vec::with_capacity(worker_count);
    for _worker in 0..worker_count {
        let storage = Arc::clone(&storage);
        let context = context.clone();
        let history_context = history_context.clone();
        let target_id = target_id.clone();
        let analysis_run_id = analysis_run_id.clone();
        workers.push(tokio::spawn(async move {
            for _round in 0..rounds_per_worker {
                let (entity, history, tests, changes) = tokio::join!(
                    storage.entity_for(&context, &target_id),
                    storage.history_entities_for(&history_context, 256),
                    storage.tests_for_context(&context, std::slice::from_ref(&target_id), 64),
                    storage.derived_relationships_for_run(&context, &analysis_run_id, 256),
                );

                assert!(
                    entity.unwrap().is_some(),
                    "stress read lost the selected Code entity"
                );
                assert!(
                    !history.unwrap().is_empty(),
                    "stress read lost History Graph entities"
                );
                let tests = tests.unwrap();
                assert!(!tests.tests.is_empty(), "stress read lost TIE tests");
                assert!(tests.tests.len() <= 64);
                assert!(tests.relationships.len() <= 64);
                let changes = changes.unwrap();
                assert!(
                    !changes.is_empty(),
                    "stress read lost Change Graph relationships"
                );
                assert!(changes.len() <= 256);
            }
        }));
    }

    tokio::time::timeout(Duration::from_secs(120), join_all(workers))
        .await
        .expect("concurrent storage reads exceeded the 120 second stress-test budget")
        .into_iter()
        .for_each(|result| result.expect("Level 3 read-stress worker panicked"));

    assert!(
        prepared
            .storage
            .verified_snapshot_exists(&context)
            .await
            .unwrap()
    );
    println!(
        "Completed {} workers x {} rounds x 4 bounded graph queries",
        worker_count, rounds_per_worker
    );
    println!("Stress-tested workspace: {}", prepared.workspace_id);
}

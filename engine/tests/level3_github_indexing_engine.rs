//! Ultimate Level 3 test: Rust repository with unit/integration test sources.

mod support;
#[path = "ultimate_github/support.rs"]
mod ultimate_support;

use codemind_engine::source::Language;

#[tokio::test]
#[ignore = "requires outbound GitHub access, the git executable, and a running FalkorDB instance"]
async fn indexing_engine_runs_through_clone_all_graphs_verification_cleanup_and_reuse() {
    ultimate_support::run_remote_repository(ultimate_support::RemoteRepositoryCase {
        slug: "nizaam-indexing-engine",
        url: "https://github.com/nizaam-islamic-ecosystem/indexing-engine.git",
        expected_languages: &[Language::Rust],
        minimum_static_test_nodes: 1,
        require_ipynb_as_incomplete: false,
    })
    .await;
}

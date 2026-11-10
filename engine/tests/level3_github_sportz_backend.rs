//! Ultimate Level 3 test: JavaScript backend with an existing JS test file.

mod support;
#[path = "ultimate_github/support.rs"]
mod ultimate_support;

use codemind_engine::source::Language;

#[tokio::test]
#[ignore = "requires outbound GitHub access, the git executable, and a running FalkorDB instance"]
async fn sportz_backend_runs_through_clone_all_graphs_verification_cleanup_and_reuse() {
    ultimate_support::run_remote_repository(ultimate_support::RemoteRepositoryCase {
        slug: "sportz-websockets-backend",
        url: "https://github.com/0xshariq/sportz-websockets-backend.git",
        expected_languages: &[Language::JavaScript],
        minimum_static_test_nodes: 1,
        require_ipynb_as_incomplete: false,
    })
    .await;
}

//! Ultimate Level 3 test: TypeScript + TSX public GitHub repository.

mod support;
#[path = "ultimate_github/support.rs"]
mod ultimate_support;

use codemind_engine::source::Language;

#[tokio::test]
#[ignore = "requires outbound GitHub access, the git executable, and a running FalkorDB instance"]
async fn coinpulse_runs_through_clone_all_graphs_verification_cleanup_and_reuse() {
    ultimate_support::run_remote_repository(ultimate_support::RemoteRepositoryCase {
        slug: "coinpulse",
        url: "https://github.com/0xshariq/coinpulse.git",
        expected_languages: &[Language::TypeScript, Language::Tsx],
        minimum_static_test_nodes: 0,
        require_ipynb_as_incomplete: false,
    })
    .await;
}

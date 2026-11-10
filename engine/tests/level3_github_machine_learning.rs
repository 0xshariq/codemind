//! Ultimate Level 3 test: machine-learning repository with Python source and Jupyter notebooks.

mod support;
#[path = "ultimate_github/support.rs"]
mod ultimate_support;

use codemind_engine::source::Language;

#[tokio::test]
#[ignore = "requires outbound GitHub access, the git executable, and a running FalkorDB instance"]
async fn machine_learning_runs_through_clone_all_graphs_verification_cleanup_and_reuse() {
    ultimate_support::run_remote_repository(ultimate_support::RemoteRepositoryCase {
        slug: "machine-learning",
        url: "https://github.com/0xshariq/machine-learning.git",
        expected_languages: &[Language::Python],
        minimum_static_test_nodes: 0,
        // This repository intentionally contains .ipynb files. They must be
        // captured but reported as unsupported source analysis until a
        // notebook adapter exists.
        require_ipynb_as_incomplete: true,
    })
    .await;
}

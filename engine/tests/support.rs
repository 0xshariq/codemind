use codemind_engine::{
    engines::{
        chie::ChieEngine,
        coie::model::{CodeGraph, EntityKind, RelationshipKind},
        tie::{
            TieEngine,
            mapping::target_mapping,
            model::{TestEntity, TestRelationshipKind},
        },
    },
    graph_storage::FalkorDbStorage,
    source::{RepositoryContext, SourceProvider, SourceSnapshot},
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

pub fn github_repository_url() -> String {
    std::env::var("CODEMIND_GITHUB_REPOSITORY")
        .unwrap_or_else(|_| "https://github.com/0xshariq/codemind".to_owned())
}

pub fn github_reference() -> String {
    std::env::var("CODEMIND_GITHUB_REF").unwrap_or_else(|_| "main".to_owned())
}

pub fn fetch_github_repository() -> (TempDir, PathBuf) {
    let repository = github_repository_url();

    let repo = repository
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .split("github.com/")
        .nth(1)
        .expect("CODEMIND_GITHUB_REPOSITORY must point to github.com/owner/repository");

    let mut parts = repo.split('/');
    let owner = parts.next().expect("GitHub owner is required");
    let name = parts.next().expect("GitHub repository name is required");
    let reference = github_reference();

    let temp = tempfile::tempdir().expect("temporary directory");
    let archive = temp.path().join("repository.tar.gz");

    let api_url = format!("https://api.github.com/repos/{owner}/{name}/tarball/{reference}");

    let mut command = Command::new("curl");

    command
        .arg("--fail")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--retry")
        .arg("3")
        .arg("--header")
        .arg("Accept: application/vnd.github+json")
        .arg("--header")
        .arg("User-Agent: CodeMind-Level3-Test")
        .arg("--output")
        .arg(&archive)
        .arg(&api_url);

    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.trim().is_empty()
    {
        command = Command::new("curl");

        command
            .arg("--fail")
            .arg("--silent")
            .arg("--show-error")
            .arg("--location")
            .arg("--retry")
            .arg("3")
            .arg("--header")
            .arg("Accept: application/vnd.github+json")
            .arg("--header")
            .arg("User-Agent: CodeMind-Level3-Test")
            .arg("--header")
            .arg(format!("Authorization: Bearer {token}"))
            .arg("--output")
            .arg(&archive)
            .arg(&api_url);
    }

    let status = command
        .status()
        .expect("curl is required for GitHub Level 3 tests");

    assert!(status.success(), "GitHub repository download failed");

    let status = Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(temp.path())
        .status()
        .expect("tar is required for GitHub Level 3 tests");

    assert!(status.success(), "GitHub repository extraction failed");

    let root = std::fs::read_dir(temp.path())
        .expect("extracted repository directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .expect("GitHub archive did not contain a repository directory");

    (temp, root)
}

pub fn github_snapshot() -> (TempDir, PathBuf, SourceSnapshot) {
    let (temp, root) = fetch_github_repository();
    let provider = SourceProvider::default();

    let snapshot = provider
        .resolve_with_repository_identity(&root, "codemind-level3", Some(&github_repository_url()))
        .expect("GitHub repository must be captured into an immutable snapshot");

    (temp, root, snapshot)
}

pub async fn storage(graph_name: &str) -> FalkorDbStorage {
    let url = std::env::var("FALKORDB_URL").unwrap_or_else(|_| "redis://127.0.0.1:6380".to_owned());

    FalkorDbStorage::connect(&url, graph_name)
        .await
        .expect("FalkorDB must be available for Level 3 tests")
}

pub fn first_callable_symbol_name(graph: &CodeGraph) -> String {
    for relationship in graph
        .relationships
        .iter()
        .filter(|relationship| relationship.kind == RelationshipKind::Calls)
    {
        let Some(target) = graph
            .entities
            .iter()
            .find(|entity| entity.id == relationship.to && entity.kind == EntityKind::Symbol)
        else {
            continue;
        };

        let count = graph
            .entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Symbol && entity.name == target.name)
            .count();

        if count == 1 {
            return target.name.clone();
        }
    }

    graph
        .entities
        .iter()
        .find(|entity| entity.kind == EntityKind::Symbol)
        .map(|entity| entity.name.clone())
        .expect("CoIE graph must contain a symbol")
}

pub fn first_callable_symbol(graph: &CodeGraph) -> String {
    graph
        .relationships
        .iter()
        .find(|relationship| relationship.kind == RelationshipKind::Calls)
        .map(|relationship| relationship.to.clone())
        .or_else(|| {
            graph
                .entities
                .iter()
                .find(|entity| entity.kind == EntityKind::Symbol)
                .map(|entity| entity.id.clone())
        })
        .expect("CoIE graph must contain at least one symbol")
}

pub fn mapped_test_graph(
    snapshot: &SourceSnapshot,
    code_graph: &CodeGraph,
) -> (
    Vec<TestEntity>,
    Vec<codemind_engine::engines::tie::model::TestRelationship>,
) {
    let (tests, relationships) = TieEngine
        .discover(snapshot)
        .expect("TIE must parse the same GitHub snapshot");

    target_mapping::resolve_graph(code_graph, tests, relationships)
}

pub fn test_target_ids(
    relationships: &[codemind_engine::engines::tie::model::TestRelationship],
) -> Vec<String> {
    relationships
        .iter()
        .filter(|relationship| relationship.kind == TestRelationshipKind::Targets)
        .filter_map(|relationship| relationship.entity_id.clone())
        .collect()
}

pub fn assert_snapshot_provenance(graph: &CodeGraph) {
    let repository = graph
        .entities
        .iter()
        .find(|entity| entity.kind == EntityKind::Repository)
        .expect("CoIE must publish a repository provenance node");

    let snapshot = graph
        .entities
        .iter()
        .find(|entity| entity.kind == EntityKind::RepositorySnapshot)
        .expect("CoIE must publish a repository snapshot provenance node");

    assert!(graph.relationships.iter().any(|relationship| {
        relationship.kind == RelationshipKind::Contains
            && relationship.from == repository.id
            && relationship.to == snapshot.id
    }));
}

pub fn assert_safe_relative_path(path: &Path) {
    assert!(codemind_engine::source::limits::is_safe_relative(path));
}

pub fn context(snapshot: &SourceSnapshot) -> RepositoryContext {
    snapshot.context.clone()
}

pub fn analyze_with_chie(
    graph: &CodeGraph,
    tests: &[TestEntity],
    relationships: &[codemind_engine::engines::tie::model::TestRelationship],
    selected: &str,
) -> codemind_engine::engines::chie::model::ChangeAnalysis {
    ChieEngine::default()
        .analyze(graph, tests, relationships, selected)
        .expect("ChIE must consume canonical CoIE and TIE facts")
}

#[cfg(test)]
mod helper_usage {
    use super::*;

    #[test]
    fn support_helpers_are_referenced() {
        let _ = github_repository_url;
        let _ = github_reference;
        let _ = fetch_github_repository;
        let _ = github_snapshot;
        let _ = storage;
        let _ = first_callable_symbol_name;
        let _ = first_callable_symbol;
        let _ = mapped_test_graph;
        let _ = test_target_ids;
        let _ = assert_snapshot_provenance;
        let _ = assert_safe_relative_path;
        let _ = context;
        let _ = analyze_with_chie;
    }
}

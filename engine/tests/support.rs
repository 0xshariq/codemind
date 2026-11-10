//! Shared Level 3 integration-test support.
//!
//! This module contains setup and utility functions only. It deliberately has
//! no `#[test]` functions. Every Level 3 test uses the same local-checkout,
//! repository-selection, FalkorDB-adapter and `~/.codemind` conventions.

#![allow(dead_code)]

use codemind_engine::{
    config::DEFAULT_FALKORDB_URL,
    engines::{
        coie::{
            CoieEngine,
            model::{CodeGraph, EntityKind},
        },
        history::{HistoryEntity, HistoryRelationship},
        pipeline::{InvestigationPipeline, PipelineError, PipelineResult, VerifiedPipelineRequest},
        tie::{
            TieEngine,
            mapping::target_mapping,
            model::{
                TestEntity, TestNodeKind, TestRelationship, TestRelationshipKind, TestResolution,
            },
        },
    },
    git::{GitCommandConfig, open_local_repository},
    graph_storage::{FalkorDbStorage, GraphStorage},
    repository::{CodeMindPaths, RepositoryIdentity, RepositoryKnowledge, RepositoryManager},
    source::{SourceProvider, SourceSnapshot, identity::digest_text},
};
use serde::Serialize;
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct TestTarget {
    pub entity_id: String,
    pub name: String,
    pub path: String,
    pub line: u32,
}

/// All inputs and durable handles for one real CodeMind repository run.
pub struct PreparedCodeMind {
    pub root: PathBuf,
    pub run_id: String,
    pub workspace_id: String,
    pub operation_id: String,
    pub request_id: String,
    pub trace_id: String,
    pub selected_target: TestTarget,
    pub storage: Arc<FalkorDbStorage>,
    pub manager: Arc<RepositoryManager<FalkorDbStorage>>,
    pub knowledge: RepositoryKnowledge,
}

/// Resolve the existing checkout containing this engine crate.
///
/// By default the engine is expected at `<repository>/engine`. Set
/// `CODEMIND_LEVEL3_REPOSITORY` to override the checkout root. A local Git
/// checkout is mandatory; no network download or generated source fixture is
/// used by Level 3 acceptance tests.
pub fn codemind_repository_root() -> PathBuf {
    let candidate = std::env::var_os("CODEMIND_LEVEL3_REPOSITORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("the engine manifest directory must have a repository parent")
                .to_path_buf()
        });
    let root = candidate.canonicalize().unwrap_or_else(|error| {
        panic!(
            "cannot canonicalize CodeMind checkout {candidate:?}: {error}; set CODEMIND_LEVEL3_REPOSITORY to the checkout root"
        )
    });

    assert!(
        root.join(".git").exists(),
        "Level 3 requires the existing CodeMind Git checkout at {}; set CODEMIND_LEVEL3_REPOSITORY if the checkout layout differs",
        root.display()
    );
    assert!(
        root.join("engine/Cargo.toml").is_file() || root.join("Cargo.toml").is_file(),
        "{} is not a CodeMind repository root: expected engine/Cargo.toml or Cargo.toml",
        root.display()
    );
    root
}

pub fn unique_run_id() -> String {
    Uuid::new_v4().simple().to_string()
}

// The two repository-wide concurrency scenarios are expensive. Serialize the
// outer scenarios to avoid running four full repository indexings at once;
// each scenario still launches its own requests concurrently, preserving the
// concurrency behavior under test.
static LEVEL3_HEAVY_SCENARIO_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn acquire_heavy_level3_scenario_lock() -> tokio::sync::MutexGuard<'static, ()> {
    LEVEL3_HEAVY_SCENARIO_LOCK.lock().await
}

/// Small context fixture for transport-contract edge tests that do not
/// create or access persisted repository data.
pub fn repository_context(
    repository_id: &str,
    workspace_id: &str,
    snapshot_id: &str,
    graph_version: &str,
) -> codemind_engine::source::RepositoryContext {
    codemind_engine::source::RepositoryContext {
        repository_id: repository_id.to_owned(),
        workspace_id: workspace_id.to_owned(),
        source_snapshot_id: snapshot_id.to_owned(),
        graph_version: graph_version.to_owned(),
    }
}

pub fn test_repository_context(snapshot_id: &str) -> codemind_engine::source::RepositoryContext {
    repository_context("repo", "workspace", snapshot_id, "v1")
}

pub fn falkordb_url() -> String {
    std::env::var("FALKORDB_URL").unwrap_or_else(|_| DEFAULT_FALKORDB_URL.to_owned())
}

/// The namespace is a configured adapter namespace, not a caller-selected
/// physical graph. The adapter derives a physical graph from workspace_id.
pub fn falkordb_namespace() -> String {
    std::env::var("FALKORDB_GRAPH").unwrap_or_else(|_| "codemind".to_owned())
}

pub async fn connect_falkordb() -> FalkorDbStorage {
    FalkorDbStorage::connect(&falkordb_url(), falkordb_namespace())
        .await
        .unwrap_or_else(|error| {
            panic!(
                "could not connect to FalkorDB through CodeMind's FalkorDbStorage adapter at {}: {error}",
                falkordb_url()
            )
        })
}

pub async fn connect_falkordb_with(
    url: &str,
    namespace: &str,
) -> Result<FalkorDbStorage, codemind_engine::graph_storage::StorageError> {
    FalkorDbStorage::connect(url, namespace).await
}

pub fn repository_manager(
    storage: Arc<FalkorDbStorage>,
) -> Arc<RepositoryManager<FalkorDbStorage>> {
    let paths = CodeMindPaths::discover()
        .expect("CodeMind must resolve the current user's home directory for ~/.codemind state");
    Arc::new(RepositoryManager::new(
        storage,
        paths,
        SourceProvider::default(),
        GitCommandConfig::default(),
    ))
}

pub async fn repository_identity(root: &Path) -> RepositoryIdentity {
    let repository = open_local_repository(root, &GitCommandConfig::default())
        .await
        .unwrap_or_else(|error| {
            panic!(
                "cannot open local CodeMind checkout {}: {error}",
                root.display()
            )
        });
    RepositoryIdentity::from_local_directory_with_origin(root, repository.remote_locator())
        .unwrap_or_else(|error| {
            panic!(
                "cannot derive a stable identity for {}: {error}",
                root.display()
            )
        })
}

pub async fn local_snapshot(root: &Path, workspace_id: &str) -> SourceSnapshot {
    let identity = repository_identity(root).await;
    SourceProvider::default()
        .resolve_with_repository_identity(root, workspace_id, Some(identity.repository_id()))
        .unwrap_or_else(|error| panic!("cannot capture local CodeMind source snapshot: {error}"))
}

/// Discover TIE from an actual repository snapshot and enforce its generated
/// SHA-256 identity contract before a Level 3 indexing scenario proceeds.
pub fn assert_source_tie_identity_contract(snapshot: &SourceSnapshot) {
    let (tests, relationships) = TieEngine
        .discover(snapshot)
        .unwrap_or_else(|error| panic!("cannot discover TIE records in source snapshot: {error}"));
    assert_tie_identity_contract(&tests, &relationships);
}

/// Validate the random SHA-256 identity contract for a complete CoIE projection.
pub fn assert_code_graph_identity_contract(graph: &CodeGraph) {
    assert_random_sha256_ids(
        graph.entities.iter().map(|entity| entity.id.as_str()),
        "coie-entity:",
        "CoIE entity",
    );
    assert_random_sha256_ids(
        graph
            .relationships
            .iter()
            .map(|relationship| relationship.id.as_str()),
        "coie-relationship:",
        "CoIE relationship",
    );
}

/// Validate random SHA-256 identities in a persisted or in-memory History projection.
pub fn assert_history_graph_identity_contract(
    entities: &[HistoryEntity],
    relationships: &[HistoryRelationship],
) {
    assert_random_sha256_ids(
        entities.iter().map(|entity| entity.id.as_str()),
        "history-entity:",
        "History entity",
    );
    assert_random_sha256_ids(
        relationships
            .iter()
            .map(|relationship| relationship.id.as_str()),
        "history-relationship:",
        "History relationship",
    );
}

fn assert_random_sha256_ids<'a>(ids: impl IntoIterator<Item = &'a str>, prefix: &str, label: &str) {
    let mut seen = HashSet::new();
    for id in ids {
        assert!(seen.insert(id), "duplicate {label} ID: {id}");
        assert_sha256_id(id, prefix, label);
    }
}

/// Enforce the production TIE identity contract in every Level 3 scenario that
/// captures the real CodeMind checkout. This checks uniqueness and the
/// canonical SHA-256 representation, including IDs read back from storage.
pub fn assert_tie_identity_contract(tests: &[TestEntity], relationships: &[TestRelationship]) {
    let mut test_ids = HashSet::with_capacity(tests.len());
    for test in tests {
        assert!(
            test_ids.insert(test.id.as_str()),
            "duplicate TIE test ID in Level 3 source snapshot: {}",
            test.id
        );
        let prefix = match test.kind {
            TestNodeKind::Test => "tie-test:",
            TestNodeKind::TestSuite => "tie-suite:",
            TestNodeKind::Fixture => "tie-fixture:",
            TestNodeKind::TestCase => "tie-case:",
        };
        assert_random_sha256_id_contract(&test.id, prefix, "TIE node");
    }

    let mut relationship_ids = HashSet::with_capacity(relationships.len());
    for relationship in relationships {
        assert!(
            relationship_ids.insert(relationship.id.as_str()),
            "duplicate TIE relationship ID in Level 3 source snapshot: {}",
            relationship.id
        );
        assert_random_sha256_id_contract(&relationship.id, "tie-edge:", "TIE relationship");
    }
}

pub fn assert_random_sha256_id_contract(id: &str, prefix: &str, label: &str) {
    assert_sha256_id(id, prefix, label);
}

fn assert_sha256_id(id: &str, prefix: &str, label: &str) {
    let digest = id
        .strip_prefix(prefix)
        .unwrap_or_else(|| panic!("{label} identity `{id}` must start with `{prefix}`"));
    assert_eq!(digest.len(), 64, "{label} identity `{id}` is not SHA-256");
    assert!(
        digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "{label} identity `{id}` must use lowercase hexadecimal",
    );
}

/// Discover all real, uniquely name-resolved symbols targeted by a discovered
/// CodeMind test. IDs belong only to this temporary extraction and are not
/// carried into the persisted graph; candidates are represented by semantics.
pub fn test_targeted_symbols(snapshot: &SourceSnapshot) -> Vec<TestTarget> {
    snapshot
        .validate()
        .expect("the selected CodeMind source snapshot must validate");
    let code_graph = CoieEngine
        .extract(snapshot)
        .expect("CoIE must extract the real CodeMind source snapshot");
    assert_code_graph_identity_contract(&code_graph);
    let (tests, relationships) = TieEngine
        .discover(snapshot)
        .expect("TIE must discover test declarations from the real source snapshot");
    assert_tie_identity_contract(&tests, &relationships);
    let (_, mapped_relationships) =
        target_mapping::resolve_graph(&code_graph, tests, relationships);

    let mut candidates = mapped_relationships
        .iter()
        .filter(|relationship| {
            relationship.kind == TestRelationshipKind::Targets
                && relationship.resolution == TestResolution::Resolved
        })
        .filter_map(|relationship| relationship.entity_id.as_deref())
        .filter_map(|entity_id| {
            code_graph
                .entities
                .iter()
                .find(|entity| entity.id == entity_id)
        })
        .filter(|entity| entity.kind == EntityKind::Symbol)
        .filter(|candidate| {
            code_graph
                .entities
                .iter()
                .filter(|entity| {
                    matches!(entity.kind, EntityKind::Symbol | EntityKind::File)
                        && entity.name == candidate.name
                })
                .count()
                == 1
        })
        .map(|entity| TestTarget {
            // This is a temporary extraction ID. Callers must resolve the
            // candidate again from the persisted CoIE projection by path/name.
            entity_id: entity.id.clone(),
            name: entity.name.clone(),
            path: entity.path.clone(),
            line: entity.line,
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        (&left.path, left.line, &left.name).cmp(&(&right.path, right.line, &right.name))
    });
    candidates.dedup_by(|left, right| {
        left.path == right.path && left.line == right.line && left.name == right.name
    });
    candidates
}

/// Select the first source-derived candidate. This is suitable for choosing
/// the request's symbol name; persisted integration scenarios should use
/// `prepare_local_codemind`, which validates candidates against the stored TIE
/// projection before returning one.
pub fn unique_test_targeted_symbol(snapshot: &SourceSnapshot) -> TestTarget {
    test_targeted_symbols(snapshot).into_iter().next().unwrap_or_else(|| {
        panic!(
            "the real CodeMind checkout contains no uniquely resolved symbol targeted by a discovered test"
        )
    })
}

/// Perform the production repository-indexing lifecycle against the local
/// CodeMind checkout. The resulting workspace graph is retained in FalkorDB.
pub async fn prepare_local_codemind(prefix: &str) -> PreparedCodeMind {
    let root = codemind_repository_root();
    let run_id = unique_run_id();
    let safe_prefix = prefix
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let workspace_id = format!("level3-{safe_prefix}-{}", &run_id);
    let operation_id = format!("level3-index-{}", run_id);
    let request_id = format!("level3-request-{}", run_id);
    let trace_id = format!("level3-trace-{}", run_id);

    let snapshot = local_snapshot(&root, &workspace_id).await;
    let candidates = test_targeted_symbols(&snapshot);
    let requested_target = candidates.first().cloned().unwrap_or_else(|| {
        panic!("the real CodeMind checkout contains no uniquely resolved symbol targeted by a discovered test")
    });
    let storage = Arc::new(connect_falkordb().await);
    let manager = repository_manager(Arc::clone(&storage));
    let root_text = root
        .to_str()
        .expect("CodeMind checkout path must be valid UTF-8");
    let mut knowledge = manager
        .ensure_indexed(
            root_text,
            &workspace_id,
            None,
            &requested_target.name,
            &operation_id,
            &request_id,
            &trace_id,
        )
        .await
        .unwrap_or_else(|error| {
            panic!("RepositoryManager failed to index the real CodeMind checkout: {error}")
        });

    // Never carry an ID from the source-only discovery pass into the stored
    // projection. Resolve every semantic candidate against persisted CoIE and
    // accept only one for which storage returns a real resolved TARGETS edge.
    // This both respects random graph IDs and ensures the Level 3 ChIE request
    // is based on a symbol with actual persisted TIE test coverage.
    let candidate_count = candidates.len();
    let mut selected_target = None;
    let mut candidate_diagnostics = Vec::new();
    for candidate in candidates {
        let persisted_entities = storage
            .exact_code_entities_by_name(&knowledge.context, &candidate.name)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "could not resolve persisted CoIE candidate `{}` ({}:{}): {error}",
                    candidate.name, candidate.path, candidate.line
                )
            });
        let Some(entity) = persisted_entities.into_iter().find(|entity| {
            entity.kind == EntityKind::Symbol
                && entity.name == candidate.name
                && entity.path == candidate.path
                && entity.line == candidate.line
        }) else {
            candidate_diagnostics.push(format!(
                "{}:{} `{}`: no matching persisted Symbol",
                candidate.path, candidate.line, candidate.name
            ));
            continue;
        };

        let stored = storage
            .tests_for_context(&knowledge.context, std::slice::from_ref(&entity.id), 1_000)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "could not read persisted TIE relationships for `{}` ({}:{}): {error}",
                    candidate.name, candidate.path, candidate.line
                )
            });
        let has_resolved_target = has_resolved_target_edge(&stored.relationships, &entity.id);
        if has_resolved_target {
            assert_tie_identity_contract(&stored.tests, &stored.relationships);
            knowledge.selected_entity_id = entity.id.clone();
            selected_target = Some(TestTarget {
                entity_id: entity.id,
                name: candidate.name,
                path: candidate.path,
                line: candidate.line,
            });
            break;
        }

        let target_edges = stored
            .relationships
            .iter()
            .filter(|relationship| relationship.kind == TestRelationshipKind::Targets)
            .take(5)
            .map(|relationship| {
                format!(
                    "entity_id={} name={} resolution={:?}",
                    relationship.entity_id.as_deref().unwrap_or("<no-id>"),
                    relationship
                        .target_symbol_name
                        .as_deref()
                        .unwrap_or("<no-name>"),
                    relationship.resolution
                )
            })
            .collect::<Vec<_>>();
        candidate_diagnostics.push(format!(
            "{}:{} `{}`: {} tests, {} relationships, target sample [{}]{}",
            candidate.path,
            candidate.line,
            candidate.name,
            stored.tests.len(),
            stored.relationships.len(),
            target_edges.join(", "),
            if stored.truncated { ", truncated" } else { "" }
        ));
    }
    let selected_target = selected_target.unwrap_or_else(|| {
        panic!(
            "RepositoryManager indexed successfully, but no persisted resolved TIE TARGETS edge matched any of {candidate_count} source-derived candidates. Inspected: {}",
            candidate_diagnostics.join("; ")
        )
    });

    PreparedCodeMind {
        root,
        run_id,
        workspace_id,
        operation_id,
        request_id,
        trace_id,
        selected_target,
        storage,
        manager,
        knowledge,
    }
}

/// A resolved TIE target is identified by its persisted CoIE endpoint ID.
/// `target_symbol_name` is optional descriptive metadata and must not be used
/// as the identity check, particularly with independently generated graph IDs.
pub fn has_resolved_target_edge(
    relationships: &[TestRelationship],
    target_entity_id: &str,
) -> bool {
    relationships.iter().any(|relationship| {
        relationship.kind == TestRelationshipKind::Targets
            && relationship.entity_id.as_deref() == Some(target_entity_id)
            && relationship.resolution == TestResolution::Resolved
    })
}

/// Execute ChIE through the verified storage-backed path using the operation
/// created by RepositoryManager. No source re-extraction or fabricated graph
/// records are involved.
pub async fn run_verified_for_entity(
    prepared: &PreparedCodeMind,
    selected_entity_id: &str,
) -> Result<PipelineResult, PipelineError> {
    InvestigationPipeline::new(Arc::clone(&prepared.storage))
        .run_verified(VerifiedPipelineRequest {
            context: prepared.knowledge.context.clone(),
            selected_entity_id: selected_entity_id.to_owned(),
            request_id: prepared.request_id.clone(),
            trace_id: prepared.trace_id.clone(),
            operation_id: prepared.operation_id.clone(),
            knowledge_completeness: prepared.knowledge.completeness,
            freshness: prepared.knowledge.freshness,
            omitted_file_count: prepared.knowledge.omitted_file_count,
            history_complete: prepared.knowledge.history_complete,
            history_omitted_commits_at_least: prepared.knowledge.history_omitted_commits_at_least,
            history_omitted_path_changes_at_least: prepared
                .knowledge
                .history_omitted_path_changes_at_least,
        })
        .await
}

pub async fn run_verified_investigation(prepared: &PreparedCodeMind) -> PipelineResult {
    run_verified_for_entity(prepared, &prepared.knowledge.selected_entity_id)
        .await
        .unwrap_or_else(|error| panic!("verified ChIE investigation failed: {error}"))
}

/// Exact physical graph name generated by the FalkorDB adapter for one
/// workspace. This helper mirrors the adapter's domain-separated contract.
pub fn physical_graph_name(workspace_id: &str) -> String {
    format!(
        "{}_{}",
        falkordb_namespace(),
        digest_text(&format!("codemind-workspace-graph-v1\0{workspace_id}"))
    )
}

/// Persist a run manifest under `~/.codemind/level3/runs`. Graph data itself
/// remains in FalkorDB; this manifest records the physical graph name and the
/// IDs needed to inspect the retained test run.
pub fn persist_run_manifest<T: Serialize>(run_id: &str, manifest: &T) -> PathBuf {
    let paths = CodeMindPaths::discover().expect("resolve ~/.codemind for Level 3 manifest");
    paths
        .ensure_base_dirs()
        .expect("create or validate the canonical ~/.codemind directory layout");

    let level3_dir = paths.root().join("level3");
    let runs_dir = level3_dir.join("runs");
    ensure_real_directory(&level3_dir)
        .unwrap_or_else(|error| panic!("cannot prepare {}: {error}", level3_dir.display()));
    ensure_real_directory(&runs_dir)
        .unwrap_or_else(|error| panic!("cannot prepare {}: {error}", runs_dir.display()));

    let path = runs_dir.join(format!("{run_id}.json"));
    let bytes =
        serde_json::to_vec_pretty(manifest).expect("Level 3 run manifest must serialize as JSON");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap_or_else(|error| {
            panic!("cannot create Level 3 manifest {}: {error}", path.display())
        });
    file.write_all(&bytes).unwrap_or_else(|error| {
        panic!("cannot write Level 3 manifest {}: {error}", path.display())
    });
    file.sync_all()
        .unwrap_or_else(|error| panic!("cannot sync Level 3 manifest {}: {error}", path.display()));
    path
}

fn ensure_real_directory(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "refusing to use a symbolic link as a Level 3 artifact directory: {}",
                path.display()
            ),
        )),
        Ok(metadata) if !metadata.is_dir() => Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            format!(
                "Level 3 artifact path is not a directory: {}",
                path.display()
            ),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => match fs::create_dir(path) {
            Ok(()) => Ok(()),
            Err(create_error) if create_error.kind() == io::ErrorKind::AlreadyExists => {
                ensure_real_directory(path)
            }
            Err(create_error) => Err(create_error),
        },
        Err(error) => Err(error),
    }
}

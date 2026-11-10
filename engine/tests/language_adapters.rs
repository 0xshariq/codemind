//! White-box validation that the real CodeMind source snapshot reaches CoIE and
//! TIE through the configured language adapters.

mod support;

use codemind_engine::{
    engines::{
        coie::{CoieEngine, model::EntityKind},
        tie::{TieEngine, graph_builder, mapping::target_mapping},
    },
    source::Language,
};
use std::collections::{HashMap, HashSet};

#[tokio::test]
#[ignore = "requires the local CodeMind Git checkout; does not require FalkorDB"]
async fn real_codemind_source_is_parsed_and_mapped_by_its_language_adapters() {
    let root = support::codemind_repository_root();
    let workspace_id = format!("level3-language-{}", support::unique_run_id());
    let snapshot = support::local_snapshot(&root, &workspace_id).await;
    let graph = CoieEngine
        .extract(&snapshot)
        .expect("CoIE must parse CodeMind source");

    let captured_by_language = snapshot
        .manifest
        .files
        .iter()
        .filter_map(|file| file.language.map(|language| (language, file.path.as_str())))
        .fold(
            HashMap::<&'static str, usize>::new(),
            |mut counts, (language, _)| {
                *counts.entry(language.stable_name()).or_default() += 1;
                counts
            },
        );
    assert!(
        captured_by_language
            .get("rust")
            .copied()
            .unwrap_or_default()
            > 0,
        "the CodeMind repository must include captured Rust source files"
    );

    let file_paths = graph
        .entities
        .iter()
        .filter(|entity| entity.kind == EntityKind::File)
        .map(|entity| entity.path.as_str())
        .collect::<HashSet<_>>();
    let symbol_entities = graph
        .entities
        .iter()
        .filter(|entity| entity.kind == EntityKind::Symbol)
        .collect::<Vec<_>>();
    assert!(!file_paths.is_empty(), "CoIE did not emit File entities");
    assert!(
        !symbol_entities.is_empty(),
        "CoIE did not emit Symbol entities"
    );
    assert!(
        symbol_entities
            .iter()
            .any(|entity| entity.language == Some(Language::Rust)),
        "the Rust adapter produced no symbol entities for the CodeMind repository"
    );

    for entity in &graph.entities {
        assert_eq!(entity.context, snapshot.context);
        if let Some(evidence) = &entity.evidence {
            assert_eq!(evidence.snapshot_id, snapshot.context.source_snapshot_id);
            assert!(evidence.start_line > 0);
            assert!(evidence.end_line >= evidence.start_line);
        }
    }

    let (tests, relationships) = TieEngine
        .discover(&snapshot)
        .expect("TIE must discover tests through the real language-specific adapters");
    assert!(
        !tests.is_empty(),
        "TIE discovered no test declarations in CodeMind"
    );

    // Match the production RepositoryIndexPipeline sequence: resolve TIE
    // relationships against the real CoIE graph, then validate uniqueness,
    // endpoints, and context before accepting the graph for persistence.
    support::assert_tie_identity_contract(&tests, &relationships);
    let (tests, relationships) = target_mapping::resolve_graph(&graph, tests, relationships);
    support::assert_tie_identity_contract(&tests, &relationships);
    graph_builder::validate_against_code_graph(&tests, &relationships, &graph)
        .expect("real CodeMind TIE output must contain unique IDs and valid CoIE references");
    assert!(tests.iter().all(|test| test.context == snapshot.context));
    assert!(
        !relationships.is_empty(),
        "TIE produced no target/declaration relationships for the real CodeMind source snapshot"
    );
    assert!(
        relationships
            .iter()
            .all(|relationship| relationship.context == snapshot.context)
    );

    support::assert_code_graph_identity_contract(&graph);
    let unique_target = support::unique_test_targeted_symbol(&snapshot);
    assert!(
        graph.entities.iter().any(|entity| {
            entity.kind == EntityKind::Symbol
                && entity.name == unique_target.name
                && entity.path == unique_target.path
        }),
        "the shared support selector must resolve to a genuine CoIE symbol by semantic identity"
    );

    println!("Captured files by language: {captured_by_language:?}");
    println!(
        "CoIE entities: {}, relationships: {}",
        graph.entities.len(),
        graph.relationships.len()
    );
    println!(
        "TIE tests: {}, relationships: {}",
        tests.len(),
        relationships.len()
    );
}

#[tokio::test]
async fn unsupported_jupyter_notebook_is_captured_but_marks_source_analysis_incomplete() {
    use codemind_engine::source::SourceProvider;
    use std::fs;

    let root = tempfile::tempdir().expect("create source fixture");
    fs::create_dir_all(root.path().join("notebooks")).unwrap();
    fs::create_dir_all(root.path().join("src")).unwrap();
    fs::write(
        root.path().join("notebooks/model.ipynb"),
        br#"{"cells":[{"cell_type":"code","execution_count":null,"metadata":{},"outputs":[],"source":["print('hello')"]}],"metadata":{},"nbformat":4,"nbformat_minor":5}"#,
    )
    .unwrap();
    fs::write(
        root.path().join("src/main.py"),
        "def main():\n    return 1\n",
    )
    .unwrap();
    fs::create_dir_all(root.path().join("assets")).unwrap();
    fs::write(root.path().join("assets/plot.png"), [0_u8, 1, 2, 3]).unwrap();

    let snapshot = SourceProvider::default()
        .resolve(
            root.path(),
            format!("unsupported-notebook-{}", support::unique_run_id()),
        )
        .expect("capture fixture source");
    snapshot
        .validate()
        .expect("captured snapshot must validate");

    let notebook = snapshot
        .manifest
        .files
        .iter()
        .find(|file| file.path == "notebooks/model.ipynb")
        .expect("notebook bytes must remain captured");
    assert_eq!(notebook.language, None, "no notebook parser is registered");
    assert_eq!(snapshot.manifest.unsupported_source_file_count(), 1);
    assert_eq!(
        snapshot
            .manifest
            .files
            .iter()
            .find(|file| file.path == "assets/plot.png")
            .and_then(|file| file.language),
        None,
        "opaque assets remain unclassified without counting as source-analysis gaps"
    );

    let graph = CoieEngine
        .extract(&snapshot)
        .expect("CoIE must retain the notebook as a generic File entity");
    assert!(graph.entities.iter().any(|entity| {
        entity.kind == EntityKind::File && entity.path == "notebooks/model.ipynb"
    }));
    assert!(!graph.entities.iter().any(|entity| {
        entity.path == "notebooks/model.ipynb" && entity.kind == EntityKind::Symbol
    }));
}

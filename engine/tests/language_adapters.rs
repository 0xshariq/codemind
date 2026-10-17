use codemind_engine::source::{Language, SourceProvider, tree_sitter};
use std::fs;

#[test]
fn supported_extensions_are_classified_without_executing_source() {
    let root = tempfile::tempdir().unwrap();
    for (name, language) in [
        ("main.rs", Language::Rust),
        ("main.js", Language::JavaScript),
        ("main.ts", Language::TypeScript),
        ("main.py", Language::Python),
    ] {
        fs::write(root.path().join(name), "not executed").unwrap();
        let snapshot = SourceProvider::default()
            .resolve(root.path(), "workspace-a")
            .unwrap();
        assert_eq!(
            snapshot
                .manifest
                .files
                .iter()
                .find(|file| file.path == name)
                .unwrap()
                .language,
            Some(language)
        );
    }
}

#[test]
fn snapshot_changes_when_source_changes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.py"), "def first():\n    pass\n").unwrap();
    let provider = SourceProvider::default();
    let first = provider.resolve(root.path(), "workspace-a").unwrap();
    fs::write(root.path().join("main.py"), "def second():\n    pass\n").unwrap();
    let second = provider.resolve(root.path(), "workspace-a").unwrap();
    assert_ne!(
        first.context.source_snapshot_id,
        second.context.source_snapshot_id
    );
}

#[test]
fn extraction_uses_pinned_snapshot_bytes() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("main.rs"), "fn original() {}\n").unwrap();
    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-a")
        .unwrap();
    fs::write(root.path().join("main.rs"), "fn changed() {}\n").unwrap();

    assert_eq!(snapshot.file_text("main.rs").unwrap(), "fn original() {}\n");
}

#[test]
fn tree_sitter_extracts_source_located_declarations_and_calls() {
    let cases = [
        (
            Language::Rust,
            "fn target() {}\nfn caller() { target(); }\n",
            "target",
        ),
        (
            Language::JavaScript,
            "function target() {}\nfunction caller() { target(); }\n",
            "target",
        ),
        (
            Language::TypeScript,
            "function target(): void {}\nfunction caller(): void { target(); }\n",
            "target",
        ),
        (
            Language::Python,
            "def target():\n    pass\ndef caller():\n    target()\n",
            "target",
        ),
    ];

    for (language, source, declaration) in cases {
        let parsed = tree_sitter::parse(language, source);
        assert!(!parsed.has_error, "{language:?} should parse");
        assert!(
            parsed
                .declarations
                .iter()
                .any(|(name, line)| name == declaration && *line == 1)
        );
        assert!(parsed.calls.iter().any(|call| call == declaration));
    }
}

#[test]
fn repository_ignore_files_are_not_indexed() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join(".git")).unwrap();
    fs::create_dir(root.path().join("ignored")).unwrap();
    fs::write(root.path().join(".gitignore"), "ignored/\n").unwrap();
    fs::write(
        root.path().join("ignored/hidden.py"),
        "def hidden():\n    pass\n",
    )
    .unwrap();
    fs::write(root.path().join("visible.py"), "def visible():\n    pass\n").unwrap();

    let snapshot = SourceProvider::default()
        .resolve(root.path(), "workspace-a")
        .unwrap();
    let paths: Vec<_> = snapshot
        .manifest
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert!(paths.contains(&"visible.py"));
    assert!(!paths.contains(&"ignored/hidden.py"));
}

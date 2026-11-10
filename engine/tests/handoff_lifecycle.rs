use codemind_engine::{
    repository::RepositoryIdentity,
    source::{Language, SourceProvider, tree_sitter},
};
use std::{fs, path::Path, process::Command};

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

fn git_at(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap_or_else(|error| panic!("git {:?} could not be started: {error}", args));
    assert!(
        output.status.success(),
        "git {:?} failed with {:?}: {}",
        args,
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[test]
fn cloned_repository_capture_excludes_git_metadata_and_survives_clone_deletion() {
    let temp = tempfile::tempdir().unwrap();
    let working = temp.path().join("working");
    fs::create_dir_all(working.join("src")).unwrap();
    git_at(&working, &["init", "--quiet"]);
    git_at(&working, &["config", "user.name", "CodeMind Test"]);
    git_at(
        &working,
        &["config", "user.email", "codemind-tests@example.invalid"],
    );
    git_at(&working, &["config", "commit.gpgsign", "false"]);

    let source = "pub fn stable() -> &'static str { \"pinned\" }\n";
    fs::write(working.join("src/lib.rs"), source).unwrap();
    // Ensure the same omission metadata is part of both clone snapshots.
    fs::write(working.join("large.bin"), vec![b'x'; 256]).unwrap();
    git_at(&working, &["add", "-A"]);
    git_at(&working, &["commit", "--quiet", "-m", "initial source"]);

    let clone_one = temp.path().join("clone-one");
    let clone_two = temp.path().join("clone-two");
    let working_path = working.to_string_lossy().into_owned();
    let clone_one_path = clone_one.to_string_lossy().into_owned();
    let clone_two_path = clone_two.to_string_lossy().into_owned();
    git_at(
        temp.path(),
        &["clone", "--quiet", &working_path, &clone_one_path],
    );
    git_at(
        temp.path(),
        &["clone", "--quiet", &working_path, &clone_two_path],
    );

    // In production this value is supplied by the validated repository layer.
    // It remains identical across clone destinations and never uses a temp path.
    let identity =
        RepositoryIdentity::from_remote("https://github.com/CodeMind-Fixture/source-snapshot.git")
            .unwrap();
    let provider = SourceProvider {
        max_file_bytes: 64,
        max_files: 100,
        max_total_bytes: 4_096,
    };
    let first = provider
        .resolve_with_repository_identity(
            &clone_one,
            "workspace-git-fixture",
            Some(identity.canonical_locator()),
        )
        .unwrap();
    let second = provider
        .resolve_with_repository_identity(
            &clone_two,
            "workspace-git-fixture",
            Some(identity.canonical_locator()),
        )
        .unwrap();

    assert_ne!(first.root, second.root);
    assert_eq!(first.context.repository_id, identity.repository_id());
    assert_eq!(first.context.repository_id, second.context.repository_id);
    assert_eq!(
        first.context.source_snapshot_id,
        second.context.source_snapshot_id
    );
    assert_eq!(first.context.graph_version, second.context.graph_version);
    assert!(first.validate().is_ok());
    assert!(second.validate().is_ok());

    assert!(
        first
            .manifest
            .files
            .iter()
            .any(|file| file.path == "src/lib.rs")
    );
    assert!(
        first
            .manifest
            .files
            .iter()
            .all(|file| !file.path.split('/').any(|component| component == ".git")),
        ".git directory contents must not enter the source manifest"
    );
    assert!(
        first
            .files
            .keys()
            .all(|path| !path.split('/').any(|component| component == ".git")),
        ".git object/config bytes must not enter pinned source content"
    );
    assert_eq!(
        first.manifest.omitted_files, second.manifest.omitted_files,
        "omission metadata should be deterministic for identical clones"
    );
    assert!(
        first
            .manifest
            .omitted_files
            .iter()
            .any(|file| file.path == "large.bin")
    );

    let commit = git_at(&clone_two, &["rev-parse", "HEAD"]);
    assert!(!commit.is_empty());
    assert_ne!(commit, first.context.source_snapshot_id);
    assert!(first.context.source_snapshot_id.starts_with("snapshot-"));

    fs::remove_dir_all(&clone_one).unwrap();
    assert!(!clone_one.exists());
    assert!(first.validate().is_ok());
    assert_eq!(first.file_text("src/lib.rs").unwrap(), source);
}

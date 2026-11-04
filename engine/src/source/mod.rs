pub mod identity;
pub mod language;
pub mod limits;
pub mod manifest;
pub mod provider;
pub mod snapshot;
pub mod tree_sitter;

pub use language::Language;
pub use manifest::{FileEntry, OmittedFile, SourceManifest, SourceManifestError};
pub use provider::{SourceProvider, SourceProviderError};
pub use snapshot::{RepositoryContext, SnapshotValidationError, SourceSnapshot};

// Level 2 module-local integration tests belong here: they verify that source
// provider, manifest, snapshot identity, context and Tree-sitter work together.
#[cfg(test)]
mod tests {
    use super::{Language, SourceProvider, identity::graph_version_for_snapshot, tree_sitter};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn source_capture_manifest_snapshot_and_parser_form_one_consistent_slice() {
        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(
            root.path().join("src/lib.rs"),
            "fn target() {}\nfn caller() { target(); }\n",
        )
        .unwrap();
        fs::write(root.path().join("large.bin"), vec![b'x'; 128]).unwrap();
        fs::write(root.path().join("ignored.py"), "def ignored(): pass\n").unwrap();
        fs::write(root.path().join(".gitignore"), "ignored.py\n").unwrap();

        let provider = SourceProvider {
            max_file_bytes: 64,
            max_files: 20,
            max_total_bytes: 512,
        };
        let snapshot = provider
            .resolve_with_repository_identity(root.path(), "workspace-1", Some("repo-key-1"))
            .unwrap();
        snapshot.validate().unwrap();

        let file = snapshot
            .manifest
            .files
            .iter()
            .find(|entry| entry.path == "src/lib.rs")
            .unwrap();
        assert_eq!(file.language, Some(Language::Rust));
        assert_eq!(
            file.bytes,
            snapshot.file_bytes("src/lib.rs").unwrap().len() as u64
        );
        assert_eq!(
            snapshot.manifest.recomputed_snapshot_id(),
            snapshot.context.source_snapshot_id
        );
        assert_eq!(
            graph_version_for_snapshot(&snapshot.context.source_snapshot_id),
            snapshot.context.graph_version
        );
        assert!(
            snapshot
                .manifest
                .omitted_files
                .iter()
                .any(|entry| entry.path == "large.bin")
        );
        assert!(
            !snapshot
                .manifest
                .files
                .iter()
                .any(|entry| entry.path == "ignored.py")
        );

        let parsed = tree_sitter::parse(Language::Rust, snapshot.file_text("src/lib.rs").unwrap());
        assert!(!parsed.has_error);
        assert_eq!(parsed.language, Some(Language::Rust));
        assert!(!parsed.adapter_version.is_empty());
        assert!(
            parsed
                .declaration_details
                .iter()
                .any(|declaration| declaration.name == "target")
        );
        assert!(
            parsed.call_site_details.iter().any(|call| {
                call.caller.as_deref() == Some("caller") && call.callee == "target"
            })
        );
    }

    #[test]
    fn source_identity_changes_when_the_indexed_or_omitted_set_changes() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("main.py"), "def first():\n    return 1\n").unwrap();
        fs::write(root.path().join("large.bin"), vec![b'x'; 128]).unwrap();
        let provider = SourceProvider {
            max_file_bytes: 32,
            max_files: 20,
            max_total_bytes: 512,
        };
        let first = provider
            .resolve_with_repository_identity(root.path(), "workspace-1", Some("logical-repo"))
            .unwrap();

        // The contents remain omitted, but the changed known size changes the
        // partial-input manifest and therefore its identity.
        fs::write(root.path().join("large.bin"), vec![b'y'; 129]).unwrap();
        let second = provider
            .resolve_with_repository_identity(root.path(), "workspace-1", Some("logical-repo"))
            .unwrap();
        assert_ne!(
            first.context.source_snapshot_id,
            second.context.source_snapshot_id
        );
        assert_ne!(first.context.graph_version, second.context.graph_version);
        assert_eq!(first.context.repository_id, second.context.repository_id);
    }
    #[test]
    fn captured_snapshot_remains_usable_after_source_root_is_deleted() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("main.rs"), "fn retained() {}\n").unwrap();

        let snapshot = SourceProvider::default()
            .resolve(root.path(), "workspace-snapshot-lifetime")
            .unwrap();
        let original_file = root.path().join("main.rs");
        assert!(original_file.exists());
        drop(root);

        assert!(!original_file.exists());
        assert_eq!(snapshot.file_text("main.rs").unwrap(), "fn retained() {}\n");
        snapshot.validate().unwrap();
    }

    #[test]
    fn snapshot_reads_only_pinned_bytes_and_rejects_noncanonical_paths() {
        use super::{FileEntry, RepositoryContext, SourceManifest, SourceSnapshot};
        use std::collections::BTreeMap;
        use std::path::PathBuf;

        let bytes = b"fn main() {}".to_vec();
        let digest = super::identity::digest_bytes(&bytes);
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let manifest = SourceManifest {
            repository_id: "repo".into(),
            source_snapshot_id: "snapshot".into(),
            files: vec![FileEntry {
                path: "src/main.rs".into(),
                language: Some(Language::Rust),
                bytes: bytes.len() as u64,
                digest,
            }],
            omitted_files: Vec::new(),
        };
        let snapshot = SourceSnapshot {
            root: PathBuf::from("/temporary/checkout"),
            context,
            manifest,
            files: BTreeMap::from([("src/main.rs".into(), bytes)]),
        };

        assert_eq!(snapshot.file_text("src/main.rs").unwrap(), "fn main() {}");
        assert_eq!(
            snapshot.file_bytes("src/../main.rs").unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
}

#[cfg(test)]
mod tree_sitter_tests {

    use super::{Language, tree_sitter::parse};

    #[test]
    fn captures_source_ranges_for_declarations_and_calls() {
        let parsed = parse(
            Language::Rust,
            "fn target() {}\nfn caller() { target(); }\n",
        );
        assert!(!parsed.has_error);
        assert!(parsed.declaration_details.iter().any(|item| {
            item.name == "target"
                && item.range.start_line == 1
                && item.range.end_byte > item.range.start_byte
        }));
        assert!(parsed.call_site_details.iter().any(|call| {
            call.caller.as_deref() == Some("caller")
                && call.callee == "target"
                && call.range.start_line == 2
        }));
    }

    #[test]
    fn malformed_source_has_explicit_diagnostics() {
        let parsed = parse(Language::Python, "def broken(:\n    pass\n");
        assert!(parsed.has_error);
        assert!(!parsed.diagnostics.is_empty());
    }

    #[test]
    fn recognizes_imports_in_supported_grammar_families() {
        let rust = parse(Language::Rust, "use std::fmt;\nfn f() {}\n");
        let python = parse(
            Language::Python,
            "from pathlib import Path\ndef f():\n    pass\n",
        );
        assert!(
            rust.import_details
                .iter()
                .any(|item| item.text.contains("std::fmt"))
        );
        assert!(
            python
                .import_details
                .iter()
                .any(|item| item.text.contains("pathlib"))
        );
    }
}

#[cfg(test)]
mod provider_integration_tests {
    use super::{SourceProvider, provider::SourceProviderError};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn captures_pinned_source_and_records_oversized_files() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.path().join("large.bin"), vec![b'x'; 128]).unwrap();
        let provider = SourceProvider {
            max_file_bytes: 32,
            max_files: 20,
            max_total_bytes: 256,
        };
        let snapshot = provider.resolve(root.path(), "workspace-a").unwrap();
        assert_eq!(snapshot.file_text("main.rs").unwrap(), "fn main() {}\n");
        assert_eq!(snapshot.manifest.omitted_files.len(), 1);
        assert_eq!(snapshot.manifest.omitted_files[0].path, "large.bin");
        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn prunes_builtin_ignored_directories_before_counting_walk_entries() {
        let root = tempdir().unwrap();
        fs::create_dir(root.path().join("node_modules")).unwrap();
        fs::create_dir(root.path().join("node_modules/pkg")).unwrap();
        fs::write(root.path().join("node_modules/pkg/generated.js"), "ignored").unwrap();
        fs::write(root.path().join("main.rs"), "fn main() {}\n").unwrap();

        let provider = SourceProvider {
            max_file_bytes: 64,
            max_files: 10,
            max_total_bytes: 256,
        };
        let snapshot = provider.resolve(root.path(), "workspace-a").unwrap();

        assert!(snapshot.files.contains_key("main.rs"));
        assert!(
            !snapshot
                .files
                .keys()
                .any(|path| path.starts_with("node_modules/"))
        );
        assert!(
            !snapshot
                .manifest
                .omitted_files
                .iter()
                .any(|entry| entry.path.starts_with("node_modules/"))
        );
    }

    #[test]
    fn total_byte_budget_omits_files_without_publishing_them_as_captured() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("a.py"), "def a(): pass\n").unwrap();
        fs::write(root.path().join("b.py"), "def b(): pass\n").unwrap();
        let provider = SourceProvider {
            max_file_bytes: 64,
            max_files: 10,
            max_total_bytes: 16,
        };
        let snapshot = provider.resolve(root.path(), "workspace-a").unwrap();
        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.manifest.omitted_files.len(), 1);
        assert!(
            snapshot.manifest.omitted_files[0]
                .reason
                .contains("remaining total source-byte budget")
        );
        assert!(
            snapshot
                .files
                .values()
                .map(|bytes| bytes.len() as u64)
                .sum::<u64>()
                <= 16
        );
        assert!(snapshot.validate().is_ok());
    }

    #[test]
    fn stable_repository_identity_survives_a_different_checkout_path() {
        let first_root = tempdir().unwrap();
        let second_root = tempdir().unwrap();
        for root in [first_root.path(), second_root.path()] {
            fs::write(root.join("lib.py"), "def f():\n    return 1\n").unwrap();
        }
        let provider = SourceProvider::default();
        let first = provider
            .resolve_with_repository_identity(first_root.path(), "workspace", Some("repo-key"))
            .unwrap();
        let second = provider
            .resolve_with_repository_identity(second_root.path(), "workspace", Some("repo-key"))
            .unwrap();
        assert_eq!(first.context.repository_id, second.context.repository_id);
        assert_eq!(
            first.context.source_snapshot_id,
            second.context.source_snapshot_id
        );
        assert_eq!(first.context.graph_version, second.context.graph_version);
    }

    #[test]
    fn invalid_limits_and_non_directory_roots_are_explicit_errors() {
        let root = tempdir().unwrap();
        let invalid = SourceProvider {
            max_file_bytes: 0,
            ..SourceProvider::default()
        };
        assert!(matches!(
            invalid.resolve(root.path(), "workspace"),
            Err(SourceProviderError::InvalidConfiguration(_))
        ));
        let file = root.path().join("file.txt");
        fs::write(&file, "text").unwrap();
        assert!(matches!(
            SourceProvider::default().resolve(&file, "workspace"),
            Err(SourceProviderError::NotDirectory(_))
        ));
    }
}

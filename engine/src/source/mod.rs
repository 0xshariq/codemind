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
}

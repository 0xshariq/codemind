pub mod evidence;
pub mod normalize;
pub mod resolve;
pub mod validate;

use crate::{
    engines::coie::model::ResolutionStatus,
    source::{Language, RepositoryContext, tree_sitter::SourceRange},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub path: String,
    pub language: Language,
    pub native_kind: String,
    pub line: u32,
    pub range: SourceRange,
    pub context: RepositoryContext,
    pub enclosing: Option<String>,
    pub resolution: ResolutionStatus,
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::source::Language;
    use std::path::Path;

    fn candidate(name: &str, path: &str, enclosing: Option<&str>) -> Candidate {
        Candidate {
            name: name.into(),
            path: path.into(),
            language: Language::Rust,
            native_kind: "function_item".into(),
            line: 1,
            range: SourceRange {
                start_byte: 0,
                end_byte: 3,
                start_line: 1,
                end_line: 1,
                start_column: 1,
                end_column: 4,
            },
            context: RepositoryContext {
                repository_id: "repo".into(),
                workspace_id: "workspace".into(),
                source_snapshot_id: "snapshot".into(),
                graph_version: "graph".into(),
            },
            enclosing: enclosing.map(str::to_owned),
            resolution: ResolutionStatus::Resolved,
        }
    }

    #[test]
    fn extraction_helpers_canonicalize_evidence_resolve_and_validate_candidates() {
        let candidates = vec![
            candidate("target", "src/lib.rs", None),
            candidate("target", "src/lib.rs", Some("mod")),
        ];
        assert!(normalize::identifier(" target ").is_some());
        assert!(validate::supported(&candidates[0]));
        assert_eq!(
            resolve::in_scope(&candidates, "src/lib.rs", Some("mod"), "target").len(),
            1
        );
        assert_eq!(
            resolve::status(&candidates, "missing"),
            ResolutionStatus::Unresolved
        );
        let evidence = evidence::from_range("src/lib.rs", 2, 4, "snapshot", "test");
        assert_eq!(evidence.start_line, 2);
        assert!(crate::source::limits::is_safe_relative(Path::new(
            &candidates[0].path
        )));
    }
}

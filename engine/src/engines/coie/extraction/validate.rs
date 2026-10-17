use super::Candidate;
use crate::{engines::coie::model::ResolutionStatus, source::limits};
use std::path::Path;

pub fn supported(candidate: &Candidate) -> bool {
    !candidate.name.trim().is_empty()
        && limits::is_safe_relative(Path::new(&candidate.path))
        && !candidate.native_kind.trim().is_empty()
        && candidate.line > 0
        && candidate.range.start_line > 0
        && candidate.range.end_line >= candidate.range.start_line
        && candidate.range.start_byte <= candidate.range.end_byte
        && candidate.context.validate().is_ok()
        && !matches!(candidate.resolution, ResolutionStatus::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::supported;
    use crate::engines::coie::extraction::Candidate;
    use crate::engines::coie::model::ResolutionStatus;
    use crate::source::{Language, RepositoryContext, tree_sitter::SourceRange};

    #[test]
    fn validation_rejects_unsafe_paths_and_unsupported_resolution() {
        let base = Candidate {
            name: "target".into(),
            path: "src/lib.rs".into(),
            language: Language::Rust,
            native_kind: "function_item".into(),
            line: 1,
            range: SourceRange {
                start_byte: 0,
                end_byte: 1,
                start_line: 1,
                end_line: 1,
                start_column: 1,
                end_column: 2,
            },
            context: RepositoryContext {
                repository_id: "repo".into(),
                workspace_id: "workspace".into(),
                source_snapshot_id: "snapshot".into(),
                graph_version: "graph".into(),
            },
            enclosing: None,
            resolution: ResolutionStatus::Resolved,
        };
        assert!(supported(&base));
        let unsafe_path = Candidate {
            path: "../secret.rs".into(),
            ..base.clone()
        };
        assert!(!supported(&unsafe_path));
        let unsupported = Candidate {
            resolution: ResolutionStatus::Unsupported,
            ..base
        };
        assert!(!supported(&unsupported));
    }
}

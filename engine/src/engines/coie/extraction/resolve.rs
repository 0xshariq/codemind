use super::Candidate;
use crate::engines::coie::model::ResolutionStatus;

pub fn by_name<'a>(candidates: &'a [Candidate], name: &str) -> Option<&'a Candidate> {
    let mut matches = candidates.iter().filter(|candidate| candidate.name == name);
    let first = matches.next()?;
    matches.next().is_none().then_some(first)
}

pub fn in_path<'a>(candidates: &'a [Candidate], path: &str, name: &str) -> Vec<&'a Candidate> {
    candidates
        .iter()
        .filter(|c| c.path == path && c.name == name)
        .collect()
}

pub fn in_scope<'a>(
    candidates: &'a [Candidate],
    path: &str,
    enclosing: Option<&str>,
    name: &str,
) -> Vec<&'a Candidate> {
    candidates
        .iter()
        .filter(|c| c.path == path && c.enclosing.as_deref() == enclosing && c.name == name)
        .collect()
}

pub fn status(candidates: &[Candidate], name: &str) -> ResolutionStatus {
    match candidates
        .iter()
        .filter(|candidate| candidate.name == name)
        .count()
    {
        0 => ResolutionStatus::Unresolved,
        1 => ResolutionStatus::Resolved,
        _ => ResolutionStatus::Ambiguous,
    }
}

pub fn status_in_scope(
    candidates: &[Candidate],
    path: &str,
    enclosing: Option<&str>,
    name: &str,
) -> ResolutionStatus {
    match in_scope(candidates, path, enclosing, name).len() {
        0 => ResolutionStatus::Unresolved,
        1 => ResolutionStatus::Resolved,
        _ => ResolutionStatus::Ambiguous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::coie::extraction::Candidate;
    use crate::engines::coie::model::ResolutionStatus;
    use crate::source::{Language, RepositoryContext, tree_sitter::SourceRange};

    fn candidate(name: &str, enclosing: Option<&str>) -> Candidate {
        Candidate {
            name: name.into(),
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
            enclosing: enclosing.map(str::to_owned),
            resolution: ResolutionStatus::Resolved,
        }
    }

    #[test]
    fn resolution_distinguishes_unique_missing_and_ambiguous_candidates() {
        let values = vec![
            candidate("target", None),
            candidate("target", Some("scope")),
        ];
        assert_eq!(status(&values, "missing"), ResolutionStatus::Unresolved);
        assert_eq!(status(&values, "target"), ResolutionStatus::Ambiguous);
        assert_eq!(
            in_scope(&values, "src/lib.rs", Some("scope"), "target").len(),
            1
        );
        assert!(by_name(&[candidate("unique", None)], "unique").is_some());
    }
}

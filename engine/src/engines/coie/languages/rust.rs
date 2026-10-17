use crate::{
    engines::coie::{extraction::Candidate, model::ResolutionStatus},
    source::{Language, RepositoryContext, tree_sitter},
};

pub fn declarations(source: &str, path: &str, context: &RepositoryContext) -> Vec<Candidate> {
    tree_sitter::parse(Language::Rust, source)
        .declaration_details
        .into_iter()
        .map(|d| Candidate {
            name: d.name,
            path: path.to_owned(),
            language: Language::Rust,
            native_kind: d.node_kind,
            line: d.range.start_line,
            range: d.range,
            context: context.clone(),
            enclosing: d.enclosing_declaration,
            resolution: ResolutionStatus::Resolved,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::declarations;
    use crate::source::{Language, RepositoryContext};

    #[test]
    fn adapter_emits_a_resolved_candidate_with_source_context() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let candidates = declarations("fn rust_fn() {}", "src/test.rs", &context);
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].language, Language::Rust);
        assert_eq!(candidates[0].context, context);
    }
}

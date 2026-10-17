use crate::{
    engines::coie::{extraction::Candidate, model::ResolutionStatus},
    source::{Language, RepositoryContext, tree_sitter},
};

pub fn declarations(
    language: Language,
    source: &str,
    path: &str,
    context: &RepositoryContext,
) -> Vec<Candidate> {
    assert!(matches!(language, Language::TypeScript | Language::Tsx));
    tree_sitter::parse(language, source)
        .declaration_details
        .into_iter()
        .map(|d| Candidate {
            name: d.name,
            path: path.to_owned(),
            language,
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
    fn adapter_preserves_tsx_language_identity() {
        let context = RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        };
        let candidates = declarations(
            Language::Tsx,
            "function view() { return null; }",
            "src/view.tsx",
            &context,
        );
        assert_eq!(candidates[0].language, Language::Tsx);
        assert_eq!(candidates[0].path, "src/view.tsx");
    }
}

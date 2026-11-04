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

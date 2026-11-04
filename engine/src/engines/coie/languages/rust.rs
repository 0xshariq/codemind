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

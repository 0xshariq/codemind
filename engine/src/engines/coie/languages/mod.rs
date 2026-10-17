use crate::{
    engines::coie::extraction::Candidate,
    source::{Language, RepositoryContext, tree_sitter},
};

pub mod javascript;
pub mod python;
pub mod rust;
pub mod typescript;

pub fn supports(language: Language) -> bool {
    matches!(
        language,
        Language::Rust
            | Language::JavaScript
            | Language::TypeScript
            | Language::Tsx
            | Language::Python
    )
}

pub fn parse(
    language: Language,
    source: &str,
    path: &str,
    context: &RepositoryContext,
) -> tree_sitter::ParsedSource {
    debug_assert!(supports(language));
    let _ = (path, context);
    tree_sitter::parse(language, source)
}

pub fn candidates(
    parsed: &tree_sitter::ParsedSource,
    path: &str,
    context: &RepositoryContext,
) -> Vec<Candidate> {
    parsed
        .declaration_details
        .iter()
        .map(|declaration| Candidate {
            name: declaration.name.clone(),
            path: path.to_owned(),
            language: parsed.language.expect("parser always records language"),
            native_kind: declaration.node_kind.clone(),
            line: declaration.range.start_line,
            range: declaration.range,
            context: context.clone(),
            enclosing: declaration.enclosing_declaration.clone(),
            resolution: crate::engines::coie::model::ResolutionStatus::Resolved,
        })
        .collect()
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn context() -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn all_supported_adapters_feed_the_same_candidate_contract() {
        let samples = [
            (Language::Rust, "fn rust_fn() {}", "src/lib.rs"),
            (Language::JavaScript, "function js_fn() {}", "src/lib.js"),
            (Language::TypeScript, "function ts_fn() {}", "src/lib.ts"),
            (Language::Tsx, "function tsx_fn() {}", "src/view.tsx"),
            (Language::Python, "def py_fn():\n    pass", "src/lib.py"),
        ];
        for (language, source, path) in samples {
            let parsed = parse(language, source, path, &context());
            let candidates = candidates(&parsed, path, &context());
            assert!(supports(language));
            assert!(!candidates.is_empty(), "no candidate for {path}");
            assert_eq!(candidates[0].language, language);
        }
    }
}

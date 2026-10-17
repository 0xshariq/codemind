use super::model::{
    TestEntity, TestRelationship, TestRelationshipKind, TestResolution, TestStatus,
};
use crate::source::{Language, SourceSnapshot, tree_sitter};

pub mod javascript_tests;
pub mod python_tests;
pub mod rust_tests;
pub mod typescript_tests;

pub(crate) use crate::source::tree_sitter::{node_text, parse_tree, range_of};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestDeclaration {
    pub name: String,
    pub framework: String,
    pub status: TestStatus,
    pub range: crate::source::tree_sitter::SourceRange,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TieEngine;

impl TieEngine {
    pub fn discover(
        &self,
        snapshot: &SourceSnapshot,
    ) -> std::io::Result<(Vec<TestEntity>, Vec<TestRelationship>)> {
        let mut tests = Vec::new();
        let mut relationships = Vec::new();

        for file in &snapshot.manifest.files {
            let Some(language) = file.language else {
                continue;
            };
            let source = snapshot.file_text(&file.path)?;
            let parsed = tree_sitter::parse(language, source);
            let declarations = discover_declarations(language, source)?;

            for declaration in &declarations {
                let test_id = format!(
                    "test:{}:{}:{}:{}",
                    snapshot.context.source_snapshot_id,
                    file.path,
                    declaration.range.start_line,
                    declaration.range.start_byte
                );
                tests.push(TestEntity {
                    id: test_id.clone(),
                    name: declaration.name.clone(),
                    path: file.path.clone(),
                    line: declaration.range.start_line,
                    framework: declaration.framework.clone(),
                    status: declaration.status,
                    context: snapshot.context.clone(),
                });
                relationships.push(TestRelationship {
                    id: format!("declared-by:{test_id}"),
                    kind: TestRelationshipKind::DeclaredBy,
                    test_id: test_id.clone(),
                    target_symbol_name: None,
                    entity_id: None,
                    evidence_path: file.path.clone(),
                    evidence: Some(crate::engines::coie::model::SourceEvidence {
                        path: file.path.clone(),
                        start_line: declaration.range.start_line,
                        end_line: declaration.range.end_line,
                        snapshot_id: snapshot.context.source_snapshot_id.clone(),
                        provenance: "tree-sitter-test-declaration".into(),
                    }),
                    resolution: TestResolution::Unresolved,
                    context: snapshot.context.clone(),
                });
            }

            for call in &parsed.call_site_details {
                let Some(declaration) = innermost_test_declaration(&declarations, call.range)
                else {
                    continue;
                };
                let short = call.callee.rsplit('.').next().unwrap_or(&call.callee);
                if matches!(short, "test" | "it" | "describe") {
                    continue;
                }
                let test_id = format!(
                    "test:{}:{}:{}:{}",
                    snapshot.context.source_snapshot_id,
                    file.path,
                    declaration.range.start_line,
                    declaration.range.start_byte
                );
                let target_name = short.to_owned();
                relationships.push(TestRelationship {
                    id: format!(
                        "targets:{test_id}:{target_name}:{}:{}",
                        call.range.start_line, call.range.start_byte
                    ),
                    kind: TestRelationshipKind::Targets,
                    test_id,
                    target_symbol_name: Some(target_name),
                    entity_id: None,
                    evidence_path: file.path.clone(),
                    evidence: Some(crate::engines::coie::model::SourceEvidence {
                        path: file.path.clone(),
                        start_line: call.range.start_line,
                        end_line: call.range.end_line,
                        snapshot_id: snapshot.context.source_snapshot_id.clone(),
                        provenance: "tree-sitter-test-body-call".into(),
                    }),
                    resolution: TestResolution::Unresolved,
                    context: snapshot.context.clone(),
                });
            }
        }

        Ok((tests, relationships))
    }
}

fn discover_declarations(
    language: Language,
    source: &str,
) -> std::io::Result<Vec<TestDeclaration>> {
    match language {
        Language::Rust => rust_tests::discover(source),
        Language::JavaScript => javascript_tests::discover(source),
        Language::TypeScript | Language::Tsx => typescript_tests::discover_for(language, source),
        Language::Python => python_tests::discover(source),
    }
}

fn innermost_test_declaration(
    declarations: &[TestDeclaration],
    range: crate::source::tree_sitter::SourceRange,
) -> Option<&TestDeclaration> {
    declarations
        .iter()
        .filter(|declaration| {
            declaration.range.start_byte <= range.start_byte
                && declaration.range.end_byte >= range.end_byte
        })
        .min_by_key(|declaration| {
            declaration
                .range
                .end_byte
                .saturating_sub(declaration.range.start_byte)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_language_discovery_modules_and_scopes_calls_to_test_ranges() {
        let rust = "fn helper() {}\n#[test]\nfn works() { helper(); }\n";
        let declarations = rust_tests::discover(rust).unwrap();
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "works");

        let parsed = tree_sitter::parse(Language::Rust, rust);
        let call = parsed
            .call_site_details
            .iter()
            .find(|call| call.callee == "helper")
            .unwrap();
        assert!(innermost_test_declaration(&declarations, call.range).is_some());
    }

    #[test]
    fn comments_and_strings_do_not_create_test_declarations() {
        let source = "// #[test]\nconst S: &str = \"#[test]\";\nfn helper() {}\n";
        assert!(rust_tests::discover(source).unwrap().is_empty());
    }
}

use super::discovery::{declaration_id, discover_declarations, innermost_test_declaration};
use super::model::{
    TestEntity, TestNodeKind, TestRelationship, TestRelationshipKind, TestResolution, TestStatus,
    TestTargetKind,
};
use crate::source::{SourceSnapshot, identity::RandomIdGenerator, tree_sitter};

/// Canonical in-memory TIE result used by discovery, validation and future
/// pipeline integration. It is deliberately the same pair returned by
/// `TieEngine::discover`, not a second graph representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestGraph {
    pub tests: Vec<TestEntity>,
    pub relationships: Vec<TestRelationship>,
}

pub fn graph(tests: Vec<TestEntity>, relationships: Vec<TestRelationship>) -> TestGraph {
    TestGraph {
        tests,
        relationships,
    }
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
        let mut id_generator = RandomIdGenerator::default();

        for file in &snapshot.manifest.files {
            let Some(language) = file.language else {
                continue;
            };
            let source = snapshot.file_text(&file.path)?;
            let parsed = tree_sitter::parse(language, source);
            let declarations = discover_declarations(language, source)?;
            let mut ids_by_declaration = std::collections::HashMap::new();
            let mut suite_ids_by_start = std::collections::HashMap::new();
            let mut fixture_ids_by_name = std::collections::HashMap::new();

            for declaration in &declarations {
                let id = declaration_id(&mut id_generator, declaration.kind);
                ids_by_declaration.insert(
                    (
                        declaration.range.start_byte,
                        declaration.range.end_byte,
                        declaration.kind,
                        declaration.name.clone(),
                        declaration.framework.clone(),
                    ),
                    id.clone(),
                );
                if declaration.kind == TestNodeKind::TestSuite {
                    suite_ids_by_start.insert(declaration.range.start_byte, id.clone());
                }
                if declaration.kind == TestNodeKind::Fixture {
                    fixture_ids_by_name.insert(declaration.name.clone(), id.clone());
                }
                tests.push(TestEntity {
                    id: id.clone(),
                    name: declaration.name.clone(),
                    path: file.path.clone(),
                    line: declaration.range.start_line,
                    framework: declaration.framework.clone(),
                    status: declaration.status,
                    kind: declaration.kind,
                    context: snapshot.context.clone(),
                });
            }

            for declaration in &declarations {
                let source_id = ids_by_declaration
                    .get(&(
                        declaration.range.start_byte,
                        declaration.range.end_byte,
                        declaration.kind,
                        declaration.name.clone(),
                        declaration.framework.clone(),
                    ))
                    .cloned()
                    .expect("every TIE declaration receives an ID");

                if declaration.kind == TestNodeKind::Test {
                    relationships.push(TestRelationship {
                        id: relationship_id(&mut id_generator, TestRelationshipKind::DeclaredBy),
                        kind: TestRelationshipKind::DeclaredBy,
                        test_id: source_id.clone(),
                        target_symbol_name: Some(declaration.name.clone()),
                        entity_id: None,
                        target_kind: TestTargetKind::Code,
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

                if let Some(parent_start_byte) = declaration.parent_start_byte
                    && let Some(parent_id) = suite_ids_by_start.get(&parent_start_byte)
                {
                    relationships.push(TestRelationship {
                        id: relationship_id(&mut id_generator, TestRelationshipKind::Contains),
                        kind: TestRelationshipKind::Contains,
                        test_id: parent_id.clone(),
                        target_symbol_name: None,
                        entity_id: Some(source_id.clone()),
                        target_kind: TestTargetKind::Test,
                        evidence_path: file.path.clone(),
                        evidence: None,
                        resolution: TestResolution::Resolved,
                        context: snapshot.context.clone(),
                    });
                }

                for case in &declaration.cases {
                    let case_id = id_generator.next("tie-case", "parameterized-test-case");
                    tests.push(TestEntity {
                        id: case_id.clone(),
                        name: case.name.clone(),
                        path: file.path.clone(),
                        line: case.range.start_line,
                        framework: declaration.framework.clone(),
                        status: TestStatus::Supported,
                        kind: TestNodeKind::TestCase,
                        context: snapshot.context.clone(),
                    });
                    relationships.push(TestRelationship {
                        id: relationship_id(&mut id_generator, TestRelationshipKind::HasCase),
                        kind: TestRelationshipKind::HasCase,
                        test_id: source_id.clone(),
                        target_symbol_name: None,
                        entity_id: Some(case_id),
                        target_kind: TestTargetKind::Test,
                        evidence_path: file.path.clone(),
                        evidence: Some(crate::engines::coie::model::SourceEvidence {
                            path: file.path.clone(),
                            start_line: case.range.start_line,
                            end_line: case.range.end_line,
                            snapshot_id: snapshot.context.source_snapshot_id.clone(),
                            provenance: "tree-sitter-parameterized-test-case".into(),
                        }),
                        resolution: TestResolution::Resolved,
                        context: snapshot.context.clone(),
                    });
                }

                for fixture_name in &declaration.fixture_names {
                    if declaration.kind == TestNodeKind::Fixture {
                        continue;
                    }
                    let Some(fixture_id) = fixture_ids_by_name.get(fixture_name) else {
                        if let Some(test) = tests.iter_mut().find(|test| test.id == source_id) {
                            test.status = TestStatus::Partial;
                        }
                        continue;
                    };
                    relationships.push(TestRelationship {
                        id: relationship_id(&mut id_generator, TestRelationshipKind::UsesFixture),
                        kind: TestRelationshipKind::UsesFixture,
                        test_id: source_id.clone(),
                        target_symbol_name: Some(fixture_name.clone()),
                        entity_id: Some(fixture_id.clone()),
                        target_kind: TestTargetKind::Test,
                        evidence_path: file.path.clone(),
                        evidence: Some(crate::engines::coie::model::SourceEvidence {
                            path: file.path.clone(),
                            start_line: declaration.range.start_line,
                            end_line: declaration.range.end_line,
                            snapshot_id: snapshot.context.source_snapshot_id.clone(),
                            provenance: "python-fixture-parameter".into(),
                        }),
                        resolution: TestResolution::Resolved,
                        context: snapshot.context.clone(),
                    });
                }

                if declaration.kind == TestNodeKind::Fixture {
                    for fixture_name in &declaration.fixture_names {
                        let Some(target_id) = fixture_ids_by_name.get(fixture_name) else {
                            continue;
                        };
                        if target_id == &source_id {
                            continue;
                        }
                        relationships.push(TestRelationship {
                            id: relationship_id(&mut id_generator, TestRelationshipKind::DependsOn),
                            kind: TestRelationshipKind::DependsOn,
                            test_id: source_id.clone(),
                            target_symbol_name: Some(fixture_name.clone()),
                            entity_id: Some(target_id.clone()),
                            target_kind: TestTargetKind::Test,
                            evidence_path: file.path.clone(),
                            evidence: Some(crate::engines::coie::model::SourceEvidence {
                                path: file.path.clone(),
                                start_line: declaration.range.start_line,
                                end_line: declaration.range.end_line,
                                snapshot_id: snapshot.context.source_snapshot_id.clone(),
                                provenance: "python-fixture-dependency".into(),
                            }),
                            resolution: TestResolution::Resolved,
                            context: snapshot.context.clone(),
                        });
                    }
                }
            }

            for call in &parsed.call_site_details {
                let Some(declaration) = innermost_test_declaration(&declarations, call.range)
                else {
                    continue;
                };
                if declaration.kind != TestNodeKind::Test {
                    continue;
                }
                let short = call.callee.rsplit('.').next().unwrap_or(&call.callee);
                if matches!(short, "test" | "it" | "describe")
                    || call.callee.contains("test.each")
                    || call.callee.contains("it.each")
                {
                    continue;
                }
                let test_id = ids_by_declaration
                    .get(&(
                        declaration.range.start_byte,
                        declaration.range.end_byte,
                        TestNodeKind::Test,
                        declaration.name.clone(),
                        declaration.framework.clone(),
                    ))
                    .cloned()
                    .expect("test declaration has an ID");
                let target_name = short.to_owned();
                relationships.push(TestRelationship {
                    id: relationship_id(&mut id_generator, TestRelationshipKind::Targets),
                    kind: TestRelationshipKind::Targets,
                    test_id,
                    target_symbol_name: Some(target_name),
                    entity_id: None,
                    target_kind: TestTargetKind::Code,
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

        // Declaration adapters already normalize duplicate AST discoveries before
        // IDs are assigned. Keep all generated records here: random IDs must not
        // be used as a semantic deduplication key.

        Ok((tests, relationships))
    }
}

fn relationship_id(generator: &mut RandomIdGenerator, kind: TestRelationshipKind) -> String {
    let domain = match kind {
        TestRelationshipKind::Contains => "contains",
        TestRelationshipKind::UsesFixture => "uses-fixture",
        TestRelationshipKind::DependsOn => "depends-on",
        TestRelationshipKind::DeclaredBy => "declared-by",
        TestRelationshipKind::Targets => "targets",
        TestRelationshipKind::HasCase => "has-case",
        TestRelationshipKind::SetupBy => "setup-by",
        TestRelationshipKind::TeardownBy => "teardown-by",
    };
    generator.next("tie-edge", domain)
}

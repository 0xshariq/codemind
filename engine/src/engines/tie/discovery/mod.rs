use super::model::{TestEntity, TestRelationship, TestRelationshipKind, TestStatus};
use crate::source::{Language, SourceSnapshot};
use std::fs;

pub mod javascript_tests;
pub mod python_tests;
pub mod rust_tests;
pub mod typescript_tests;

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
            let source = fs::read_to_string(snapshot.root.join(&file.path))?;
            for (index, line) in source.lines().enumerate() {
                let (is_test, framework) = match language {
                    Language::Rust => (line.contains("#[test]"), "rust"),
                    Language::JavaScript | Language::TypeScript => (
                        line.contains("test(") || line.contains("it("),
                        "jest-vitest",
                    ),
                    Language::Python => (
                        line.trim_start().starts_with("def test_")
                            || line.contains("unittest.TestCase"),
                        "pytest-unittest",
                    ),
                };
                if !is_test {
                    continue;
                }
                let name = line
                    .split(|character: char| {
                        !(character.is_ascii_alphanumeric() || character == '_')
                    })
                    .find(|value| value.starts_with("test") || value == &"it")
                    .unwrap_or("anonymous-test")
                    .to_owned();
                let test_id = format!(
                    "test:{}:{}:{}",
                    snapshot.context.source_snapshot_id, file.path, index
                );
                tests.push(TestEntity {
                    id: test_id.clone(),
                    name,
                    path: file.path.clone(),
                    line: index as u32 + 1,
                    framework: framework.to_owned(),
                    status: TestStatus::Supported,
                    context: snapshot.context.clone(),
                });
                relationships.push(TestRelationship {
                    kind: TestRelationshipKind::DeclaredBy,
                    test_id,
                    target_symbol_name: None,
                    evidence_path: file.path.clone(),
                    context: snapshot.context.clone(),
                });
            }
        }
        Ok((tests, relationships))
    }
}

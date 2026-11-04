use super::model::{TestNodeKind, TestStatus};
use crate::source::Language;

pub mod javascript_tests;
pub mod python_tests;
pub mod rust_tests;
pub mod typescript_tests;

pub(crate) use crate::source::tree_sitter::{node_text, parse_tree, range_of};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCaseDeclaration {
    pub name: String,
    pub range: crate::source::tree_sitter::SourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestDeclaration {
    pub name: String,
    pub framework: String,
    pub status: TestStatus,
    pub kind: TestNodeKind,
    pub range: crate::source::tree_sitter::SourceRange,
    pub parent_start_byte: Option<usize>,
    pub fixture_names: Vec<String>,
    pub cases: Vec<TestCaseDeclaration>,
}

pub(crate) fn declaration_id(
    repository_id: &str,
    path: &str,
    declaration: &TestDeclaration,
) -> String {
    let prefix = match declaration.kind {
        TestNodeKind::Test => "test",
        TestNodeKind::TestSuite => "test-suite",
        TestNodeKind::Fixture => "test-fixture",
        TestNodeKind::TestCase => "test-case",
    };
    if declaration.kind == TestNodeKind::Test {
        format!(
            "test:{repository_id}:{path}:{}:{}",
            declaration.range.start_line, declaration.range.start_byte
        )
    } else {
        format!(
            "{prefix}:{repository_id}:{path}:{}",
            declaration.range.start_byte
        )
    }
}

pub(crate) fn discover_declarations(
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

pub(crate) fn innermost_test_declaration(
    declarations: &[TestDeclaration],
    range: crate::source::tree_sitter::SourceRange,
) -> Option<&TestDeclaration> {
    declarations
        .iter()
        .filter(|declaration| {
            declaration.kind == TestNodeKind::Test
                && declaration.range.start_byte <= range.start_byte
                && declaration.range.end_byte >= range.end_byte
        })
        .min_by_key(|declaration| {
            declaration
                .range
                .end_byte
                .saturating_sub(declaration.range.start_byte)
        })
}

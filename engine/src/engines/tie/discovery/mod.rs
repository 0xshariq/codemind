use super::model::{TestNodeKind, TestStatus};
use crate::{source::Language, source::identity::RandomIdGenerator};

pub mod javascript_tests;
pub mod python_tests;
pub mod rust_tests;
pub mod typescript_tests;

pub(crate) use crate::source::tree_sitter::{node_text, parse_tree, range_of};

/// Returns true when this call expression is part of the callee chain of an
/// outer call. Tree-sitter may place member/subscript/parenthesized wrappers
/// between a parameterized factory call and the outer invocation. Examples
/// include `test.each(table)(name, body)` and
/// `test.each(table).bind(test)(name, body)`. Only the outer invocation is a
/// test declaration; the inner factory expression is not a separate test.
pub(crate) fn is_callee_of_chained_call(node: tree_sitter::Node<'_>) -> bool {
    let mut current = node;
    loop {
        let Some(parent) = current.parent() else {
            return false;
        };

        if parent.kind() == "call_expression" {
            return parent
                .child_by_field_name("function")
                .is_some_and(|function| {
                    function.start_byte() <= node.start_byte()
                        && function.end_byte() >= node.end_byte()
                });
        }

        // Continue only through expression wrappers that can legally form a
        // member/call callee. Do not walk through arguments or function bodies:
        // nested test declarations there are independent declarations.
        if matches!(
            parent.kind(),
            "member_expression"
                | "subscript_expression"
                | "parenthesized_expression"
                | "non_null_expression"
                | "instantiation_expression"
                | "as_expression"
                | "satisfies_expression"
                | "type_assertion"
        ) {
            current = parent;
            continue;
        }
        return false;
    }
}

/// Return the callable path and whether `function` is a parameterized factory
/// call used as the callee of the current invocation. The path is deliberately
/// limited to identifier/member chains; substring matching would incorrectly
/// classify calls such as `test.each(cases).forEach(...)` as test declarations.
pub(crate) fn callable_path(
    function: tree_sitter::Node<'_>,
    source: &[u8],
) -> Option<(String, bool)> {
    let (path_node, is_factory_call) = if function.kind() == "call_expression" {
        (function.child_by_field_name("function")?, true)
    } else {
        (function, false)
    };
    if !matches!(path_node.kind(), "identifier" | "member_expression") {
        return None;
    }
    let path = node_text(path_node, source)?;
    if path.is_empty()
        || !path.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|ch| ch == '_' || ch == '$' || ch.is_ascii_alphanumeric())
        })
    {
        return None;
    }
    Some((path, is_factory_call))
}

pub(crate) fn is_suite_invocation(function: tree_sitter::Node<'_>, source: &[u8]) -> bool {
    let Some((path, factory)) = callable_path(function, source) else {
        return false;
    };
    let mut parts = path.split('.');
    if parts.next() != Some("describe") {
        return false;
    }
    let modifiers = parts.collect::<Vec<_>>();
    if factory {
        modifiers.last() == Some(&"each")
            && modifiers[..modifiers.len().saturating_sub(1)]
                .iter()
                .all(|modifier| matches!(*modifier, "only" | "skip"))
    } else {
        modifiers
            .iter()
            .all(|modifier| matches!(*modifier, "only" | "skip"))
    }
}

/// Whether the current call invokes a supported `test`/`it` declaration API.
/// The returned boolean identifies parameterized `.each(...)` invocation.
pub(crate) fn is_test_invocation(function: tree_sitter::Node<'_>, source: &[u8]) -> Option<bool> {
    let (path, factory) = callable_path(function, source)?;
    let mut parts = path.split('.');
    if !matches!(parts.next(), Some("test" | "it")) {
        return None;
    }
    let modifiers = parts.collect::<Vec<_>>();
    if factory {
        if modifiers.last() != Some(&"each")
            || !modifiers[..modifiers.len().saturating_sub(1)]
                .iter()
                .all(|modifier| matches!(*modifier, "only" | "skip" | "concurrent"))
        {
            return None;
        }
        Some(true)
    } else if modifiers
        .iter()
        .all(|modifier| matches!(*modifier, "only" | "skip" | "concurrent" | "todo"))
    {
        Some(false)
    } else {
        None
    }
}

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

/// Mint a random SHA-256 ID for a TIE node. The generator is owned by one
/// discovery pass and guarantees that no node receives an ID already issued
/// in that pass.
pub(crate) fn declaration_id(generator: &mut RandomIdGenerator, kind: TestNodeKind) -> String {
    let (prefix, domain) = match kind {
        TestNodeKind::Test => ("tie-test", "test"),
        TestNodeKind::TestSuite => ("tie-suite", "test-suite"),
        TestNodeKind::Fixture => ("tie-fixture", "fixture"),
        TestNodeKind::TestCase => ("tie-case", "test-case"),
    };
    generator.next(prefix, domain)
}

pub(crate) fn discover_declarations(
    language: Language,
    source: &str,
) -> std::io::Result<Vec<TestDeclaration>> {
    let declarations = match language {
        Language::Rust => rust_tests::discover(source)?,
        Language::JavaScript => javascript_tests::discover(source)?,
        Language::TypeScript | Language::Tsx => typescript_tests::discover_for(language, source)?,
        Language::Python => python_tests::discover(source)?,
    };

    // Defensive normalization: a syntax traversal must emit each concrete
    // declaration once. If overlapping traversal paths expose the same AST
    // node more than once, retain one copy rather than publishing duplicate
    // graph entities and relationships.
    let mut seen = std::collections::HashSet::with_capacity(declarations.len());
    Ok(declarations
        .into_iter()
        .filter(|declaration| {
            seen.insert((
                declaration.kind,
                declaration.range.start_byte,
                declaration.range.end_byte,
                declaration.name.clone(),
                declaration.framework.clone(),
            ))
        })
        .collect())
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

#[cfg(test)]
mod identity_tests {
    use crate::source::identity::RandomIdGenerator;

    #[test]
    fn generated_tie_node_ids_are_random_sha256_values_and_unique_within_discovery() {
        let mut generator = RandomIdGenerator::default();
        let ids = (0..128)
            .map(|_| generator.next("tie-test", "test"))
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(), 128);
        assert_eq!(generator.issued_count(), 128);
        assert!(ids.iter().all(|id| {
            id.strip_prefix("tie-test:").is_some_and(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        }));
    }
}

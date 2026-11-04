use super::{TestDeclaration, node_text, parse_tree, range_of};
use crate::engines::tie::model::{TestNodeKind, TestStatus};

pub fn discover(source: &str) -> std::io::Result<Vec<TestDeclaration>> {
    let tree = parse_tree(crate::source::Language::Python, source)?;
    let mut declarations = Vec::new();
    collect(tree.root_node(), source.as_bytes(), None, &mut declarations);
    Ok(declarations)
}

fn collect(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    current_suite: Option<usize>,
    output: &mut Vec<TestDeclaration>,
) {
    let mut suite = current_suite;

    if node.kind() == "class_definition" && class_extends_test_case(node, source) {
        let range = range_of(node);
        let name = node
            .child_by_field_name("name")
            .and_then(|child| node_text(child, source))
            .unwrap_or_else(|| "anonymous-suite".into());
        output.push(TestDeclaration {
            name,
            framework: "unittest".into(),
            status: TestStatus::Supported,
            kind: TestNodeKind::TestSuite,
            range,
            parent_start_byte: current_suite,
            fixture_names: Vec::new(),
            cases: Vec::new(),
        });
        suite = Some(range.start_byte);
    }

    if node.kind() == "function_definition"
        && let Some(name) = node
            .child_by_field_name("name")
            .and_then(|child| node_text(child, source))
    {
        let decorator_text = decorators_text(node, source);
        if decorator_text.contains("pytest.fixture") {
            let range = range_of(node);
            output.push(TestDeclaration {
                name,
                framework: "pytest".into(),
                status: TestStatus::Supported,
                kind: TestNodeKind::Fixture,
                range,
                parent_start_byte: None,
                fixture_names: parameter_names(node, source),
                cases: Vec::new(),
            });
        } else if name.starts_with("test_") || (suite.is_some() && name.starts_with("test")) {
            let range = range_of(node);
            output.push(TestDeclaration {
                name,
                framework: if suite.is_some() {
                    "unittest".into()
                } else {
                    "pytest".into()
                },
                status: TestStatus::Supported,
                kind: TestNodeKind::Test,
                range,
                parent_start_byte: suite,
                fixture_names: parameter_names(node, source),
                cases: Vec::new(),
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(child, source, suite, output);
    }
}

fn class_extends_test_case(node: tree_sitter::Node<'_>, source: &[u8]) -> bool {
    node.child_by_field_name("superclasses")
        .and_then(|supers| node_text(supers, source))
        .is_some_and(|text| {
            text.split(',').any(|item| {
                item.trim()
                    .trim_matches(|character: char| {
                        character == '(' || character == ')' || character.is_whitespace()
                    })
                    .ends_with("TestCase")
            })
        })
}

fn decorators_text(node: tree_sitter::Node<'_>, source: &[u8]) -> String {
    let Some(body) = node.parent() else {
        return String::new();
    };
    let mut cursor = body.walk();
    let siblings: Vec<_> = body.named_children(&mut cursor).collect();
    let Some(index) = siblings.iter().position(|child| child.id() == node.id()) else {
        return String::new();
    };
    siblings[..index]
        .iter()
        .rev()
        .take_while(|child| child.kind() == "decorator")
        .filter_map(|child| node_text(*child, source))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parameter_names(node: tree_sitter::Node<'_>, source: &[u8]) -> Vec<String> {
    let Some(parameters) = node.child_by_field_name("parameters") else {
        return Vec::new();
    };
    let text = node_text(parameters, source).unwrap_or_default();
    text.trim_matches(|character: char| character == '(' || character == ')')
        .split(',')
        .filter_map(|item| {
            let name = item
                .trim()
                .split('=')
                .next()
                .unwrap_or_default()
                .trim()
                .trim_start_matches('*')
                .trim();
            if name.is_empty() || matches!(name, "self" | "cls") {
                None
            } else if name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
            {
                Some(name.to_owned())
            } else {
                None
            }
        })
        .collect()
}

pub fn is_test_declaration(source: &str) -> bool {
    discover(source)
        .map(|items| items.iter().any(|item| item.kind == TestNodeKind::Test))
        .unwrap_or(false)
}

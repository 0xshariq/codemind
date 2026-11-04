use super::{TestCaseDeclaration, TestDeclaration, node_text, parse_tree, range_of};
use crate::engines::tie::model::{TestNodeKind, TestStatus};

pub fn discover(source: &str) -> std::io::Result<Vec<TestDeclaration>> {
    let tree = parse_tree(crate::source::Language::Rust, source)?;
    let mut declarations = Vec::new();
    collect(tree.root_node(), source.as_bytes(), &mut declarations);
    Ok(declarations)
}

fn collect(node: tree_sitter::Node<'_>, source: &[u8], output: &mut Vec<TestDeclaration>) {
    if node.kind() == "function_item"
        && has_test_attribute(node, source)
        && let Some(name) = node
            .child_by_field_name("name")
            .and_then(|child| node_text(child, source))
    {
        output.push(TestDeclaration {
            name,
            framework: "rust".into(),
            status: TestStatus::Supported,
            kind: TestNodeKind::Test,
            range: range_of(node),
            parent_start_byte: None,
            fixture_names: Vec::new(),
            cases: rstest_cases(node, source),
        });
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(child, source, output);
    }
}

fn has_test_attribute(function: tree_sitter::Node<'_>, source: &[u8]) -> bool {
    let Some(parent) = function.parent() else {
        return false;
    };

    let mut cursor = parent.walk();
    let siblings: Vec<_> = parent.named_children(&mut cursor).collect();
    let Some(index) = siblings.iter().position(|node| node.id() == function.id()) else {
        return false;
    };

    siblings[..index]
        .iter()
        .rev()
        .take_while(|node| node.kind() == "attribute_item")
        .any(|node| {
            node_text(*node, source).is_some_and(|text| {
                let normalized: String = text
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect();
                normalized == "#[test]"
                    || normalized.starts_with("#[tokio::test]")
                    || normalized.starts_with("#[tokio::test(")
                    || normalized.starts_with("#[async_std::test]")
                    || normalized.starts_with("#[async_std::test(")
                    || normalized == "#[rstest]"
                    || normalized.starts_with("#[rstest(")
            })
        })
}

fn rstest_cases(function: tree_sitter::Node<'_>, source: &[u8]) -> Vec<TestCaseDeclaration> {
    let Some(parent) = function.parent() else {
        return Vec::new();
    };
    let mut cursor = parent.walk();
    let siblings: Vec<_> = parent.named_children(&mut cursor).collect();
    let Some(index) = siblings.iter().position(|node| node.id() == function.id()) else {
        return Vec::new();
    };

    let mut ranges = Vec::new();
    for attribute in siblings[..index]
        .iter()
        .rev()
        .take_while(|node| node.kind() == "attribute_item")
    {
        let Some(text) = node_text(*attribute, source) else {
            continue;
        };
        let normalized = text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        if !(normalized.starts_with("#[case(") || normalized.starts_with("#[case::"))
            || !normalized.ends_with(")]")
        {
            continue;
        }
        ranges.push(range_of(*attribute));
    }
    ranges.reverse();
    ranges
        .into_iter()
        .enumerate()
        .map(|(index, range)| TestCaseDeclaration {
            name: format!("case-{}", index + 1),
            range,
        })
        .collect()
}

pub fn is_test_declaration(source: &str) -> bool {
    discover(source)
        .map(|items| items.iter().any(|item| item.kind == TestNodeKind::Test))
        .unwrap_or(false)
}

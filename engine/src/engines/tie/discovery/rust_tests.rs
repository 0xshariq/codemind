use super::{TestDeclaration, node_text, parse_tree, range_of};
use crate::source::tree_sitter::SourceRange;

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
            status: super::TestStatus::Supported,
            range: range_of(node),
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
            })
        })
}

#[allow(dead_code)]
fn _range(_range: SourceRange) {}

pub fn is_test_declaration(source: &str) -> bool {
    discover(source)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::discover;

    #[test]
    fn discovers_supported_rust_test_attributes_without_comment_false_positives() {
        let source = "// #[test]\n#[tokio::test]\nasync fn works() { helper(); }\n";

        let tests = discover(source).unwrap();

        assert_ne!(tests.len(), 2);
        assert_eq!(tests[0].name, "works");
        assert_eq!(tests[0].framework, "rust");
        assert_eq!(tests[0].range.start_line, 3);
    }
}

use super::{TestDeclaration, node_text, parse_tree, range_of};

pub fn discover(source: &str) -> std::io::Result<Vec<TestDeclaration>> {
    let tree = parse_tree(crate::source::Language::Python, source)?;
    let mut declarations = Vec::new();
    collect(
        tree.root_node(),
        source.as_bytes(),
        false,
        &mut declarations,
    );
    Ok(declarations)
}

fn collect(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    inside_unittest: bool,
    output: &mut Vec<TestDeclaration>,
) {
    let mut unittest_scope = inside_unittest;
    if node.kind() == "class_definition" {
        unittest_scope = class_extends_test_case(node, source);
    }
    if node.kind() == "function_definition"
        && let Some(name) = node
            .child_by_field_name("name")
            .and_then(|child| node_text(child, source))
        && (name.starts_with("test_") || (inside_unittest && name != "__init__"))
    {
        output.push(TestDeclaration {
            name,
            framework: if inside_unittest {
                "unittest"
            } else {
                "pytest"
            }
            .into(),
            status: super::TestStatus::Supported,
            range: range_of(node),
        });
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(child, source, unittest_scope, output);
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

pub fn is_test_declaration(source: &str) -> bool {
    discover(source)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::discover;

    #[test]
    fn discovers_pytest_and_unittest_methods_from_ast_structure() {
        let source = "import unittest\n\ndef test_value():\n    helper()\n\nclass Tests(unittest.TestCase):\n    def test_method(self):\n        helper()\n";
        let tests = discover(source).unwrap();
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].framework, "pytest");
        assert_eq!(tests[1].framework, "unittest");
    }
}

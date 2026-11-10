use super::{
    TestCaseDeclaration, TestDeclaration, is_callee_of_chained_call, is_suite_invocation,
    is_test_invocation, node_text, parse_tree, range_of,
};
use crate::engines::tie::model::{TestNodeKind, TestStatus};

pub fn discover(source: &str) -> std::io::Result<Vec<TestDeclaration>> {
    let tree = parse_tree(crate::source::Language::JavaScript, source)?;
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
    if node.kind() == "call_expression"
        && !is_callee_of_chained_call(node)
        && let Some(function) = node.child_by_field_name("function")
    {
        let is_suite = is_suite_invocation(function, source);
        let test_invocation = is_test_invocation(function, source);

        if is_suite && let Some(arguments) = node.child_by_field_name("arguments") {
            let name = arguments
                .named_child(0)
                .and_then(|child| node_text(child, source))
                .map(|value| strip_literal(&value))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "anonymous-suite".into());

            let range = range_of(node);

            output.push(TestDeclaration {
                name,
                framework: "jest-vitest".into(),
                status: TestStatus::Supported,
                kind: TestNodeKind::TestSuite,
                range,
                parent_start_byte: current_suite,
                fixture_names: Vec::new(),
                cases: Vec::new(),
            });

            let suite_start = Some(range.start_byte);
            let mut cursor = node.walk();

            for child in node.named_children(&mut cursor) {
                collect(child, source, suite_start, output);
            }

            return;
        }

        if let Some(is_parameterized) = test_invocation
            && let Some(arguments) = node.child_by_field_name("arguments")
        {
            let name = arguments
                .named_child(0)
                .and_then(|child| node_text(child, source))
                .map(|value| strip_literal(&value))
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "anonymous-test".into());

            let status = if is_parameterized {
                TestStatus::Partial
            } else if arguments.named_child(1).is_some() {
                TestStatus::Supported
            } else {
                TestStatus::Partial
            };

            let range = range_of(node);

            output.push(TestDeclaration {
                name,
                framework: "jest-vitest".into(),
                status,
                kind: TestNodeKind::Test,
                range,
                parent_start_byte: current_suite,
                fixture_names: Vec::new(),
                cases: Vec::<TestCaseDeclaration>::new(),
            });
        }
    }

    let mut cursor = node.walk();

    for child in node.named_children(&mut cursor) {
        collect(child, source, current_suite, output);
    }
}

fn strip_literal(value: &str) -> String {
    value
        .trim_matches(|character| matches!(character, '\'' | '"' | '`'))
        .trim()
        .to_owned()
}

pub fn is_test_declaration(source: &str) -> bool {
    discover(source)
        .map(|items| items.iter().any(|item| item.kind == TestNodeKind::Test))
        .unwrap_or(false)
}

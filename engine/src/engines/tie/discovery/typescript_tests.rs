use super::{TestDeclaration, node_text, parse_tree, range_of};

pub fn discover_for(
    language: crate::source::Language,
    source: &str,
) -> std::io::Result<Vec<TestDeclaration>> {
    let tree = parse_tree(language, source)?;
    let mut declarations = Vec::new();
    collect(tree.root_node(), source.as_bytes(), &mut declarations);
    Ok(declarations)
}

fn collect(node: tree_sitter::Node<'_>, source: &[u8], output: &mut Vec<TestDeclaration>) {
    if node.kind() == "call_expression"
        && let Some(function) = node.child_by_field_name("function")
    {
        let callee = node_text(function, source).unwrap_or_default();
        let short = callee.rsplit('.').next().unwrap_or(callee.as_str());
        if matches!(short, "test" | "it")
            && let Some(arguments) = node.child_by_field_name("arguments")
        {
            let name = arguments
                .named_child(0)
                .and_then(|child| node_text(child, source))
                .map(|value| {
                    value
                        .trim_matches(|character| matches!(character, '\'' | '"' | '`'))
                        .trim()
                        .to_owned()
                })
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "anonymous-test".into());
            let status = if arguments.named_child(1).is_some() {
                super::TestStatus::Supported
            } else {
                super::TestStatus::Partial
            };
            output.push(TestDeclaration {
                name,
                framework: "jest-vitest".into(),
                status,
                range: range_of(node),
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(child, source, output);
    }
}

pub fn is_test_declaration(source: &str) -> bool {
    discover_for(crate::source::Language::TypeScript, source)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::discover_for;
    use crate::source::Language;

    #[test]
    fn uses_the_requested_typescript_grammar_for_tsx() {
        let source = "test('works', () => helper());\nconst view = <Panel />;\n";
        let tests = discover_for(Language::Tsx, source).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].framework, "jest-vitest");
    }
}

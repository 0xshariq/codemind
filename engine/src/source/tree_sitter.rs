use super::Language;
use tree_sitter::{Node, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseSummary {
    pub language: Language,
    pub bytes: usize,
    pub has_error: bool,
}

pub fn inspect(language: Language, source: &str) -> ParseSummary {
    let parsed = parse(language, source);
    ParseSummary {
        language,
        bytes: source.len(),
        has_error: parsed.has_error,
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParsedSource {
    pub declarations: Vec<(String, u32)>,
    pub imports: Vec<String>,
    pub calls: Vec<String>,
    pub has_error: bool,
}

pub fn parse(language: Language, source: &str) -> ParsedSource {
    let mut parser = Parser::new();
    let grammar = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
    };
    if parser.set_language(&grammar).is_err() {
        return ParsedSource {
            has_error: true,
            ..ParsedSource::default()
        };
    }
    let Some(tree) = parser.parse(source, None) else {
        return ParsedSource {
            has_error: true,
            ..ParsedSource::default()
        };
    };

    let mut result = ParsedSource {
        has_error: tree.root_node().has_error(),
        ..ParsedSource::default()
    };
    visit(tree.root_node(), source.as_bytes(), &mut result);
    result
}

fn visit(node: Node<'_>, source: &[u8], result: &mut ParsedSource) {
    let kind = node.kind();
    if is_declaration(kind) {
        if let Some(name) = named_field_text(node, "name", source) {
            result
                .declarations
                .push((name, node.start_position().row as u32 + 1));
        }
    } else if kind == "import_statement" || kind == "use_declaration" {
        let text = node.utf8_text(source).unwrap_or_default().to_owned();
        result.imports.push(text);
    } else if (kind == "call_expression" || kind == "call")
        && let Some(function) = node.child_by_field_name("function")
        && let Ok(name) = function.utf8_text(source)
    {
        let name = name.trim().to_owned();
        if !name.is_empty() {
            result.calls.push(name);
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(child, source, result);
    }
}

fn named_field_text(node: Node<'_>, field: &str, source: &[u8]) -> Option<String> {
    node.child_by_field_name(field)?
        .utf8_text(source)
        .ok()
        .map(str::to_owned)
}

fn is_declaration(kind: &str) -> bool {
    matches!(
        kind,
        "function_item"
            | "struct_item"
            | "trait_item"
            | "enum_item"
            | "function_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "function_definition"
            | "class_definition"
    )
}

use super::{Language, limits};
use tree_sitter::{Node, Parser};

/// Bump when the supported AST extraction/output contract changes.
pub const TREE_SITTER_ADAPTER_VERSION: &str = "codemind-ast-adapter-v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseSummary {
    pub language: Language,
    pub bytes: usize,
    pub has_error: bool,
}

/// Source range with zero-based UTF-8 byte offsets and one-based line/byte-column
/// positions. Tree-sitter columns are byte columns, not Unicode grapheme columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceRange {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u32,
    pub end_line: u32,
    pub start_column: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub code: &'static str,
    pub message: String,
    pub range: SourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDeclaration {
    pub name: String,
    pub node_kind: String,
    pub range: SourceRange,
    /// The nearest enclosing supported declaration, where known.
    pub enclosing_declaration: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedImport {
    pub text: String,
    pub node_kind: String,
    pub range: SourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCallSite {
    pub caller: Option<String>,
    pub callee: String,
    pub node_kind: String,
    pub range: SourceRange,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ParsedSource {
    /// Language/adapter metadata applies to every AST fact in this result.
    pub language: Option<Language>,
    pub adapter_version: &'static str,
    /// Compatibility fields retained for existing CoIE/TIE callers. New code
    /// should use the detailed records below so source provenance is not lost.
    pub declarations: Vec<(String, u32)>,
    pub imports: Vec<String>,
    pub calls: Vec<String>,
    pub call_sites: Vec<(Option<String>, String)>,
    pub declaration_details: Vec<ParsedDeclaration>,
    pub import_details: Vec<ParsedImport>,
    pub call_site_details: Vec<ParsedCallSite>,
    pub diagnostics: Vec<ParseDiagnostic>,
    pub diagnostics_truncated: bool,
    pub has_error: bool,
}

pub(crate) fn parse_tree(language: Language, source: &str) -> std::io::Result<tree_sitter::Tree> {
    let mut parser = Parser::new();
    let grammar = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
    };
    parser
        .set_language(&grammar)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string()))?;
    parser.parse(source, None).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Tree-sitter did not return a syntax tree",
        )
    })
}

pub(crate) fn node_text(node: Node<'_>, source: &[u8]) -> Option<String> {
    node.utf8_text(source).ok().map(str::to_owned)
}

pub fn inspect(language: Language, source: &str) -> ParseSummary {
    let parsed = parse(language, source);
    ParseSummary {
        language,
        bytes: source.len(),
        has_error: parsed.has_error,
    }
}

/// Parse supported source with the installed Tree-sitter grammar family.
/// Syntax or extraction problems are reported in the result rather than being
/// silently converted to empty successful facts. This function never executes
/// repository source.
pub fn parse(language: Language, source: &str) -> ParsedSource {
    let mut parser = Parser::new();
    let grammar = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
    };
    if parser.set_language(&grammar).is_err() {
        return failed_parse(
            language,
            source,
            "TREE_SITTER_GRAMMAR_UNAVAILABLE",
            format!("Could not initialize {}", language.grammar_name()),
        );
    }
    let Some(tree) = parser.parse(source, None) else {
        return failed_parse(
            language,
            source,
            "TREE_SITTER_PARSE_UNAVAILABLE",
            "Tree-sitter did not return a syntax tree".into(),
        );
    };

    let mut result = ParsedSource {
        language: Some(language),
        adapter_version: TREE_SITTER_ADAPTER_VERSION,
        has_error: tree.root_node().has_error(),
        ..ParsedSource::default()
    };
    collect_syntax_diagnostics(tree.root_node(), &mut result);
    visit(tree.root_node(), source.as_bytes(), &mut result, None);
    if result.has_error && result.diagnostics.is_empty() {
        push_diagnostic(
            &mut result,
            ParseDiagnostic {
                code: "TREE_SITTER_SYNTAX_ERROR",
                message: "Syntax tree contains an error that could not be localized".into(),
                range: whole_source_range(source),
            },
        );
    }
    result
}

fn failed_parse(
    language: Language,
    source: &str,
    code: &'static str,
    message: String,
) -> ParsedSource {
    let mut result = ParsedSource {
        language: Some(language),
        adapter_version: TREE_SITTER_ADAPTER_VERSION,
        has_error: true,
        ..ParsedSource::default()
    };
    push_diagnostic(
        &mut result,
        ParseDiagnostic {
            code,
            message,
            range: whole_source_range(source),
        },
    );
    result
}

fn visit(
    node: Node<'_>,
    source: &[u8],
    result: &mut ParsedSource,
    enclosing_declaration: Option<&str>,
) {
    let kind = node.kind();
    let declaration_candidate = is_named_declaration(kind) || is_function_valued_variable(node);
    let declaration_name = if declaration_candidate {
        named_field_text(node, "name", source)
    } else {
        None
    };

    if declaration_candidate {
        if let Some(name) = declaration_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
        {
            let range = range_of(node);
            result
                .declarations
                .push((name.to_owned(), range.start_line));
            result.declaration_details.push(ParsedDeclaration {
                name: name.to_owned(),
                node_kind: kind.to_owned(),
                range,
                enclosing_declaration: enclosing_declaration.map(str::to_owned),
            });
        } else {
            push_diagnostic(
                result,
                ParseDiagnostic {
                    code: "DECLARATION_NAME_UNAVAILABLE",
                    message: format!(
                        "Supported declaration node `{kind}` has no readable name field"
                    ),
                    range: range_of(node),
                },
            );
        }
    }

    if is_import(kind) {
        match node.utf8_text(source) {
            Ok(text) if !text.trim().is_empty() => {
                result.imports.push(text.to_owned());
                result.import_details.push(ParsedImport {
                    text: text.to_owned(),
                    node_kind: kind.to_owned(),
                    range: range_of(node),
                });
            }
            _ => push_diagnostic(
                result,
                ParseDiagnostic {
                    code: "IMPORT_TEXT_UNAVAILABLE",
                    message: format!("Could not read source text for import node `{kind}`"),
                    range: range_of(node),
                },
            ),
        }
    }

    if is_call_expression(kind) {
        if let Some(function) = node.child_by_field_name("function") {
            match function.utf8_text(source) {
                Ok(text) => {
                    let callee = text.trim();
                    if !callee.is_empty() {
                        let call = ParsedCallSite {
                            caller: enclosing_declaration.map(str::to_owned),
                            callee: callee.to_owned(),
                            node_kind: kind.to_owned(),
                            range: range_of(node),
                        };
                        result.calls.push(call.callee.clone());
                        result
                            .call_sites
                            .push((call.caller.clone(), call.callee.clone()));
                        result.call_site_details.push(call);
                    } else {
                        push_diagnostic(
                            result,
                            ParseDiagnostic {
                                code: "CALL_TARGET_EMPTY",
                                message: "Call expression has an empty target expression".into(),
                                range: range_of(node),
                            },
                        );
                    }
                }
                Err(_) => push_diagnostic(
                    result,
                    ParseDiagnostic {
                        code: "CALL_TARGET_UNAVAILABLE",
                        message: "Could not read source text for a call target".into(),
                        range: range_of(function),
                    },
                ),
            }
        } else {
            push_diagnostic(
                result,
                ParseDiagnostic {
                    code: "CALL_FUNCTION_FIELD_MISSING",
                    message: format!("Call node `{kind}` has no `function` field"),
                    range: range_of(node),
                },
            );
        }
    }

    let callback_context = test_callback_context(node, source);
    let next_enclosing = declaration_name
        .as_deref()
        .or(callback_context.as_deref())
        .or(enclosing_declaration);
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        visit(child, source, result, next_enclosing);
    }
}

fn test_callback_context(node: Node<'_>, source: &[u8]) -> Option<String> {
    if !matches!(node.kind(), "arrow_function" | "function_expression") {
        return None;
    }
    let parent = node.parent()?;
    if parent.kind() != "call_expression" {
        return None;
    }
    let function = parent.child_by_field_name("function")?;
    let callee = function.utf8_text(source).ok()?.trim();
    let short = callee.rsplit('.').next().unwrap_or(callee);
    if !matches!(short, "test" | "it") {
        return None;
    }
    Some(format!(
        "__codemind_test_callback:{}",
        parent.start_position().row + 1
    ))
}

fn collect_syntax_diagnostics(node: Node<'_>, result: &mut ParsedSource) {
    if node.is_error() {
        push_diagnostic(
            result,
            ParseDiagnostic {
                code: "TREE_SITTER_ERROR_NODE",
                message: "Tree-sitter identified an unrecognized or malformed syntax region".into(),
                range: range_of(node),
            },
        );
        // An ERROR node can contain many descendants. One range-qualified
        // diagnostic is more useful than duplicating each child error.
        return;
    }
    if node.is_missing() {
        push_diagnostic(
            result,
            ParseDiagnostic {
                code: "TREE_SITTER_MISSING_NODE",
                message: format!("Tree-sitter expected a `{}` node", node.kind()),
                range: range_of(node),
            },
        );
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_syntax_diagnostics(child, result);
    }
}

fn push_diagnostic(result: &mut ParsedSource, diagnostic: ParseDiagnostic) {
    if result.diagnostics.len() < limits::MAX_PARSE_DIAGNOSTICS {
        result.diagnostics.push(diagnostic);
    } else {
        result.diagnostics_truncated = true;
    }
}

pub(crate) fn range_of(node: Node<'_>) -> SourceRange {
    let start = node.start_position();
    let end = node.end_position();
    SourceRange {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: start.row.saturating_add(1).min(u32::MAX as usize) as u32,
        end_line: end.row.saturating_add(1).min(u32::MAX as usize) as u32,
        start_column: start.column.saturating_add(1).min(u32::MAX as usize) as u32,
        end_column: end.column.saturating_add(1).min(u32::MAX as usize) as u32,
    }
}

fn whole_source_range(source: &str) -> SourceRange {
    let lines = source
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        .saturating_add(1)
        .min(u32::MAX as usize) as u32;
    SourceRange {
        start_byte: 0,
        end_byte: source.len(),
        start_line: 1,
        end_line: lines,
        start_column: 1,
        end_column: source.rsplit('\n').next().map_or(1, |line| {
            line.len().saturating_add(1).min(u32::MAX as usize) as u32
        }),
    }
}

fn named_field_text(node: Node<'_>, field: &str, source: &[u8]) -> Option<String> {
    node.child_by_field_name(field)?
        .utf8_text(source)
        .ok()
        .map(str::to_owned)
}

fn is_import(kind: &str) -> bool {
    matches!(
        kind,
        "import_statement" | "import_from_statement" | "use_declaration"
    )
}

fn is_call_expression(kind: &str) -> bool {
    matches!(kind, "call_expression" | "call")
}

fn is_function_valued_variable(node: Node<'_>) -> bool {
    if node.kind() != "variable_declarator" {
        return false;
    }
    node.child_by_field_name("value").is_some_and(|value| {
        matches!(
            value.kind(),
            "arrow_function" | "function_expression" | "generator_function"
        )
    })
}

fn is_named_declaration(kind: &str) -> bool {
    matches!(
        kind,
        "function_item"
            | "function_signature_item"
            | "struct_item"
            | "union_item"
            | "trait_item"
            | "enum_item"
            | "type_item"
            | "const_item"
            | "static_item"
            | "mod_item"
            | "function_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration"
            | "method_definition"
            | "function_definition"
            | "class_definition"
    )
}

#[cfg(test)]
mod tests {
    use super::{Language, parse};

    #[test]
    fn captures_source_ranges_for_declarations_and_calls() {
        let parsed = parse(
            Language::Rust,
            "fn target() {}\nfn caller() { target(); }\n",
        );
        assert!(!parsed.has_error);
        assert!(parsed.declaration_details.iter().any(|item| {
            item.name == "target"
                && item.range.start_line == 1
                && item.range.end_byte > item.range.start_byte
        }));
        assert!(parsed.call_site_details.iter().any(|call| {
            call.caller.as_deref() == Some("caller")
                && call.callee == "target"
                && call.range.start_line == 2
        }));
    }

    #[test]
    fn malformed_source_has_explicit_diagnostics() {
        let parsed = parse(Language::Python, "def broken(:\n    pass\n");
        assert!(parsed.has_error);
        assert!(!parsed.diagnostics.is_empty());
    }

    #[test]
    fn recognizes_imports_in_supported_grammar_families() {
        let rust = parse(Language::Rust, "use std::fmt;\nfn f() {}\n");
        let python = parse(
            Language::Python,
            "from pathlib import Path\ndef f():\n    pass\n",
        );
        assert!(
            rust.import_details
                .iter()
                .any(|item| item.text.contains("std::fmt"))
        );
        assert!(
            python
                .import_details
                .iter()
                .any(|item| item.text.contains("pathlib"))
        );
    }
}

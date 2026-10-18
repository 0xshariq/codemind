use super::{
    extraction::{self, evidence, validate},
    graph_builder, languages,
    model::{
        CodeGraph, Diagnostic, EntityKind, GraphBuildReport, GraphEntity, GraphRelationship,
        RelationshipKind, ResolutionStatus, SourceEvidence,
    },
};
use crate::{
    graph_storage::{GraphStorage, StorageError},
    source::{SourceSnapshot, tree_sitter},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Default, Clone, Copy)]
pub struct CoieEngine;

impl CoieEngine {
    pub fn extract(&self, snapshot: &SourceSnapshot) -> std::io::Result<CodeGraph> {
        snapshot
            .validate()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        let mut graph = CodeGraph {
            context: snapshot.context.clone(),
            entities: Vec::new(),
            relationships: Vec::new(),
            diagnostics: Vec::new(),
        };

        let repository_id = entity_id(
            &snapshot.context.source_snapshot_id,
            &snapshot.context.repository_id,
            "repository",
        );
        let snapshot_id = entity_id(
            &snapshot.context.source_snapshot_id,
            &snapshot.context.source_snapshot_id,
            "repository-snapshot",
        );

        graph.entities.push(GraphEntity {
            id: repository_id.clone(),
            kind: EntityKind::Repository,
            name: snapshot.context.repository_id.clone(),
            path: snapshot.context.repository_id.clone(),
            line: 1,
            context: snapshot.context.clone(),
            language: None,
            native_kind: Some("repository".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        });

        graph.entities.push(GraphEntity {
            id: snapshot_id.clone(),
            kind: EntityKind::RepositorySnapshot,
            name: snapshot.context.source_snapshot_id.clone(),
            path: snapshot.context.source_snapshot_id.clone(),
            line: 1,
            context: snapshot.context.clone(),
            language: None,
            native_kind: Some("repository_snapshot".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        });

        graph.relationships.push(relationship(
            &repository_id,
            &snapshot_id,
            RelationshipKind::Contains,
            snapshot,
            None,
            ResolutionStatus::Resolved,
        ));

        let mut candidates = Vec::new();
        let mut file_ids = HashMap::new();
        let mut directory_ids = HashMap::new();
        let mut parsed_files = Vec::new();
        let mut imports_by_path: HashMap<String, Vec<(String, Option<String>)>> = HashMap::new();
        let mut parsed_imports_by_path = Vec::new();

        for entry in &snapshot.manifest.files {
            let file_id = entity_id(&snapshot.context.source_snapshot_id, &entry.path, "file");
            file_ids.insert(entry.path.clone(), file_id.clone());
            graph.entities.push(GraphEntity {
                id: file_id.clone(),
                kind: EntityKind::File,
                name: entry.path.clone(),
                path: entry.path.clone(),
                line: 1,
                context: snapshot.context.clone(),
                language: entry.language,
                native_kind: Some("file".into()),
                range: None,
                resolution: ResolutionStatus::Resolved,
                evidence: Some(evidence::from_location(
                    &entry.path,
                    1,
                    &snapshot.context.source_snapshot_id,
                )),
            });

            self.add_directory_chain(
                snapshot,
                &snapshot_id,
                &entry.path,
                &mut graph,
                &mut directory_ids,
            );

            let components: Vec<_> = Path::new(&entry.path).components().collect();
            if components.len() == 1 {
                graph.relationships.push(relationship(
                    &snapshot_id,
                    &file_id,
                    RelationshipKind::Contains,
                    snapshot,
                    None,
                    ResolutionStatus::Resolved,
                ));
            }

            if let Some(parent) = Path::new(&entry.path)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                && let Some(parent_id) =
                    directory_ids.get(&parent.to_string_lossy().replace('\\', "/"))
            {
                graph.relationships.push(relationship(
                    parent_id,
                    &file_id,
                    RelationshipKind::Contains,
                    snapshot,
                    None,
                    ResolutionStatus::Resolved,
                ));
            }

            let Some(language) = entry.language else {
                continue;
            };

            let source = snapshot.file_text(&entry.path)?;
            let parsed = languages::parse(language, source, &entry.path, &snapshot.context);
            self.append_parser_diagnostics(&mut graph, &entry.path, &parsed);

            let file_candidates = languages::candidates(&parsed, &entry.path, &snapshot.context);

            for candidate in &file_candidates {
                if !validate::supported(candidate) {
                    graph.diagnostics.push(Diagnostic {
                        code: "CANDIDATE_INVALID".into(),
                        message: format!("Unsupported or invalid declaration `{}`", candidate.name),
                        path: entry.path.clone(),
                        severity: "warning".into(),
                    });
                    continue;
                }

                let id = symbol_id(snapshot, candidate);

                let source_evidence = evidence::from_range(
                    &candidate.path,
                    candidate.range.start_line,
                    candidate.range.end_line,
                    &snapshot.context.source_snapshot_id,
                    tree_sitter::TREE_SITTER_ADAPTER_VERSION,
                );

                graph.entities.push(GraphEntity {
                    id: id.clone(),
                    kind: EntityKind::Symbol,
                    name: candidate.name.clone(),
                    path: candidate.path.clone(),
                    line: candidate.line,
                    context: snapshot.context.clone(),
                    language: Some(candidate.language),
                    native_kind: Some(candidate.native_kind.clone()),
                    range: Some(candidate.range),
                    resolution: candidate.resolution.clone(),
                    evidence: Some(source_evidence.clone()),
                });

                let file_id = file_ids
                    .get(&entry.path)
                    .expect("file entity inserted before symbols");

                graph.relationships.push(relationship(
                    file_id,
                    &id,
                    RelationshipKind::Defines,
                    snapshot,
                    Some(source_evidence.clone()),
                    ResolutionStatus::Resolved,
                ));

                candidates.push(candidate.clone());
            }

            let mut imported_targets: Vec<(String, Option<String>)> = Vec::new();

            for import in &parsed.import_details {
                let targets = resolve_import_paths(snapshot, &entry.path, language, &import.text);
                let imported_symbol = imported_symbol(language, &import.text);

                match targets.as_slice() {
                    [target_path] => {
                        imported_targets.push((target_path.clone(), imported_symbol.clone()));

                        // Import relationships are emitted after every file ID has been
                        // registered so repository file order cannot drop valid edges.
                    }

                    [] => graph.diagnostics.push(Diagnostic {
                        code: "IMPORT_UNRESOLVED".into(),
                        message: format!(
                            "Import `{}` could not be mapped to a captured repository file",
                            import.text
                        ),
                        path: entry.path.clone(),
                        severity: "info".into(),
                    }),

                    _ => {
                        imported_targets.extend(
                            targets
                                .iter()
                                .cloned()
                                .map(|target| (target, imported_symbol.clone())),
                        );

                        graph.diagnostics.push(Diagnostic {
                            code: "IMPORT_AMBIGUOUS".into(),
                            message: format!(
                                "Import `{}` maps to multiple captured repository files",
                                import.text
                            ),
                            path: entry.path.clone(),
                            severity: "info".into(),
                        });
                    }
                }
            }

            imported_targets.sort();
            imported_targets.dedup();

            imports_by_path.insert(entry.path.clone(), imported_targets);
            parsed_imports_by_path.push((
                entry.path.clone(),
                language,
                parsed.import_details.clone(),
            ));
            parsed_files.push((entry.path.clone(), parsed));
        }

        for (path, language, imports) in parsed_imports_by_path {
            let Some(file_id) = file_ids.get(&path) else {
                continue;
            };
            for import in imports {
                let targets = resolve_import_paths(snapshot, &path, language, &import.text);
                if let [target_path] = targets.as_slice()
                    && let Some(target_id) = file_ids.get(target_path)
                {
                    let ev = evidence::from_range(
                        &path,
                        import.range.start_line,
                        import.range.end_line,
                        &snapshot.context.source_snapshot_id,
                        tree_sitter::TREE_SITTER_ADAPTER_VERSION,
                    );
                    graph.relationships.push(relationship(
                        file_id,
                        target_id,
                        RelationshipKind::Imports,
                        snapshot,
                        Some(ev),
                        ResolutionStatus::Resolved,
                    ));
                }
            }
        }

        for (path, parsed) in parsed_files {
            for call in parsed.call_site_details {
                let caller_candidates: Vec<_> = call
                    .caller
                    .as_deref()
                    .map(|caller| {
                        candidates
                            .iter()
                            .filter(|candidate| candidate.path == path && candidate.name == caller)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let caller = match caller_candidates.as_slice() {
                    [caller] => *caller,

                    [] => {
                        graph.diagnostics.push(Diagnostic {
                            code: "CALL_CALLER_UNRESOLVED".into(),
                            message: format!(
                                "Caller `{}` could not be resolved in {}",
                                call.caller.as_deref().unwrap_or("<file>"),
                                path
                            ),
                            path: path.clone(),
                            severity: "info".into(),
                        });
                        continue;
                    }

                    _ => {
                        graph.diagnostics.push(Diagnostic {
                            code: "CALL_CALLER_AMBIGUOUS".into(),
                            message: format!(
                                "Caller `{}` is ambiguous in {}",
                                call.caller.as_deref().unwrap_or("<file>"),
                                path
                            ),
                            path: path.clone(),
                            severity: "info".into(),
                        });
                        continue;
                    }
                };

                let caller_id = symbol_id(snapshot, caller);
                let short = call.callee.rsplit('.').next().unwrap_or(&call.callee);

                let scoped: Vec<_> = candidates
                    .iter()
                    .filter(|c| {
                        c.path == path && c.name == short && c.enclosing == caller.enclosing
                    })
                    .collect();

                let local: Vec<_> = if scoped.is_empty() && caller.enclosing.is_none() {
                    candidates
                        .iter()
                        .filter(|c| c.path == path && c.name == short && c.enclosing.is_none())
                        .collect()
                } else {
                    scoped
                };

                let targets = if !local.is_empty() {
                    local
                } else {
                    let imported = imports_by_path.get(&path).cloned().unwrap_or_default();

                    candidates
                        .iter()
                        .filter(|candidate| {
                            candidate.name == short
                                && imported.iter().any(|(target, symbol)| {
                                    target == &candidate.path
                                        && symbol
                                            .as_ref()
                                            .map(|name| name == &candidate.name)
                                            .unwrap_or(true)
                                })
                        })
                        .collect::<Vec<_>>()
                };

                if targets.len() != 1 {
                    graph.diagnostics.push(Diagnostic {
                        code: if targets.is_empty() {
                            "CALL_UNRESOLVED"
                        } else {
                            "CALL_AMBIGUOUS"
                        }
                        .into(),

                        message: if targets.is_empty() {
                            format!(
                                "Call `{}` could not be resolved in the caller's local or imported modules",
                                call.callee
                            )
                        } else {
                            format!(
                                "Call `{}` has {} candidate targets in the caller's scope/imports",
                                call.callee,
                                targets.len()
                            )
                        },

                        path: path.clone(),
                        severity: "info".into(),
                    });

                    continue;
                }

                let target = symbol_id(snapshot, targets[0]);

                let ev = evidence::from_range(
                    &path,
                    call.range.start_line,
                    call.range.end_line,
                    &snapshot.context.source_snapshot_id,
                    tree_sitter::TREE_SITTER_ADAPTER_VERSION,
                );

                let duplicate = graph.relationships.iter().any(|relationship| {
                    relationship.kind == RelationshipKind::Calls
                        && relationship.from == caller_id
                        && relationship.to == target
                        && relationship.evidence.as_ref() == Some(&ev)
                });

                if !duplicate {
                    graph.relationships.push(relationship(
                        &caller_id,
                        &target,
                        RelationshipKind::Calls,
                        snapshot,
                        Some(ev),
                        ResolutionStatus::Resolved,
                    ));
                }
            }
        }

        graph.diagnostics.extend(
            snapshot
                .manifest
                .omitted_files
                .iter()
                .map(|item| Diagnostic {
                    code: "SOURCE_OMITTED".into(),
                    message: format!(
                        "Source file `{}` was omitted during bounded capture: {}",
                        item.path, item.reason
                    ),
                    path: item.path.clone(),
                    severity: "warning".into(),
                }),
        );

        graph_builder::validate_detailed(&graph).map_err(|errors| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, errors.join("; "))
        })?;

        Ok(graph)
    }

    fn add_directory_chain(
        &self,
        snapshot: &SourceSnapshot,
        snapshot_id: &str,
        file_path: &str,
        graph: &mut CodeGraph,
        ids: &mut HashMap<String, String>,
    ) {
        let mut current = PathBuf::new();

        let components: Vec<_> = Path::new(file_path).components().collect();

        for component in components.iter().take(components.len().saturating_sub(1)) {
            current.push(component.as_os_str());

            let path = current.to_string_lossy().replace('\\', "/");

            if ids.contains_key(&path) {
                continue;
            }

            let id = entity_id(&snapshot.context.source_snapshot_id, &path, "directory");

            ids.insert(path.clone(), id.clone());

            graph.entities.push(GraphEntity {
                id,
                kind: EntityKind::Directory,
                name: path.clone(),
                path: path.clone(),
                line: 1,
                context: snapshot.context.clone(),
                language: None,
                native_kind: Some("directory".into()),
                range: None,
                resolution: ResolutionStatus::Resolved,
                evidence: None,
            });

            if let Some(parent) = Path::new(&path)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
            {
                let parent_path = parent.to_string_lossy().replace('\\', "/");

                if let (Some(parent_id), Some(child_id)) = (ids.get(&parent_path), ids.get(&path)) {
                    graph.relationships.push(relationship(
                        parent_id,
                        child_id,
                        RelationshipKind::Contains,
                        snapshot,
                        None,
                        ResolutionStatus::Resolved,
                    ));
                }
            } else {
                let directory_id = ids
                    .get(&path)
                    .expect("directory entity inserted before root relationship");

                graph.relationships.push(relationship(
                    snapshot_id,
                    directory_id,
                    RelationshipKind::Contains,
                    snapshot,
                    None,
                    ResolutionStatus::Resolved,
                ));
            }
        }
    }

    fn append_parser_diagnostics(
        &self,
        graph: &mut CodeGraph,
        path: &str,
        parsed: &tree_sitter::ParsedSource,
    ) {
        for diagnostic in &parsed.diagnostics {
            graph.diagnostics.push(Diagnostic {
                code: diagnostic.code.into(),
                message: diagnostic.message.clone(),
                path: path.into(),
                severity: "warning".into(),
            });
        }

        if parsed.diagnostics_truncated {
            graph.diagnostics.push(Diagnostic {
                code: "PARSE_DIAGNOSTICS_TRUNCATED".into(),
                message: "Parser diagnostics were bounded and truncated".into(),
                path: path.into(),
                severity: "warning".into(),
            });
        }
    }

    pub async fn ingest<S: GraphStorage>(
        &self,
        snapshot: &SourceSnapshot,
        storage: &S,
    ) -> Result<GraphBuildReport, StorageError> {
        let graph = self
            .extract(snapshot)
            .map_err(|error| StorageError::Query(error.to_string()))?;

        storage.write_code_graph(&graph).await?;

        Ok(GraphBuildReport::from_graph(&graph))
    }
}

fn symbol_id(snapshot: &SourceSnapshot, candidate: &extraction::Candidate) -> String {
    let scope = candidate.enclosing.as_deref().unwrap_or("<module>");

    entity_id(
        &snapshot.context.source_snapshot_id,
        &format!(
            "{}:{}:{}:{}:{}:{}",
            candidate.path,
            candidate.range.start_byte,
            candidate.range.end_byte,
            candidate.name,
            candidate.native_kind,
            scope
        ),
        "symbol",
    )
}

fn entity_id(snapshot: &str, value: &str, kind: &str) -> String {
    format!(
        "{kind}:{snapshot}:{}",
        crate::source::identity::digest_text(value)
    )
}

fn relationship(
    from: &str,
    to: &str,
    kind: RelationshipKind,
    snapshot: &SourceSnapshot,
    evidence: Option<SourceEvidence>,
    resolution: ResolutionStatus,
) -> GraphRelationship {
    GraphRelationship {
        id: edge_id(from, to, &kind, evidence.as_ref()),
        kind,
        from: from.to_owned(),
        to: to.to_owned(),
        context: snapshot.context.clone(),
        resolution,
        evidence,
    }
}

fn edge_id(
    from: &str,
    to: &str,
    kind: &RelationshipKind,
    evidence: Option<&SourceEvidence>,
) -> String {
    let evidence_key = evidence
        .map(|e| format!("{}:{}:{}", e.path, e.start_line, e.end_line))
        .unwrap_or_default();

    format!(
        "edge:{:?}:{}",
        kind,
        crate::source::identity::digest_text(&format!("{from}:{to}:{evidence_key}"))
    )
}

fn resolve_import_paths(
    snapshot: &SourceSnapshot,
    current_path: &str,
    language: crate::source::Language,
    import_text: &str,
) -> Vec<String> {
    let candidates = import_specifiers(current_path, language, import_text);

    let mut resolved = Vec::new();

    for specifier in candidates {
        for candidate in import_path_candidates(current_path, language, &specifier) {
            if snapshot.files.contains_key(&candidate)
                && !resolved.iter().any(|existing| existing == &candidate)
            {
                resolved.push(candidate);
            }
        }
    }

    resolved
}

fn imported_symbol(language: crate::source::Language, text: &str) -> Option<String> {
    if language != crate::source::Language::Rust {
        return None;
    }

    let value = text
        .strip_prefix("use ")
        .unwrap_or(text)
        .trim_end_matches(';')
        .trim();

    // Brace imports name multiple symbols, so this helper deliberately leaves
    // them unconstrained here. The path resolver still bounds the target files.
    if value.contains('{') || value.contains('}') || value.contains(',') {
        return None;
    }

    let value = value
        .split_once(" as ")
        .map(|(path, _)| path)
        .unwrap_or(value)
        .trim();

    value
        .rsplit("::")
        .next()
        .filter(|name| !name.is_empty() && *name != "self" && *name != "super")
        .map(str::to_owned)
}

fn import_specifiers(
    _current_path: &str,
    language: crate::source::Language,
    text: &str,
) -> Vec<String> {
    let text = text.trim();

    match language {
        crate::source::Language::JavaScript
        | crate::source::Language::TypeScript
        | crate::source::Language::Tsx => {
            let mut values = Vec::new();

            if let Some(start) = text.find(['\'', '"']) {
                let quote = text.as_bytes()[start] as char;

                if let Some(end) = text[start + 1..].find(quote) {
                    values.push(text[start + 1..start + 1 + end].to_owned());
                }
            }

            values
        }

        crate::source::Language::Rust => {
            let value = text
                .strip_prefix("use ")
                .unwrap_or(text)
                .trim_end_matches(';')
                .trim();

            vec![value.to_owned()]
        }

        crate::source::Language::Python => {
            if let Some(rest) = text.strip_prefix("from ") {
                let module = rest.split_whitespace().next().unwrap_or_default();

                if !module.is_empty() {
                    return vec![module.to_owned()];
                }
            }

            if let Some(rest) = text.strip_prefix("import ") {
                return rest
                    .split(',')
                    .filter_map(|item| item.split_whitespace().next())
                    .filter(|item| !item.is_empty())
                    .map(str::to_owned)
                    .collect();
            }

            Vec::new()
        }
    }
}

fn import_path_candidates(
    current_path: &str,
    language: crate::source::Language,
    specifier: &str,
) -> Vec<String> {
    let current_parent = Path::new(current_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));

    let raw_paths = match language {
        crate::source::Language::Rust => {
            let full = rust_module_path(current_path, specifier);

            let mut paths = vec![full.clone()];

            if let Some(parent) = full.parent() {
                paths.push(parent.to_path_buf());
            }

            paths
        }

        crate::source::Language::Python => {
            vec![python_module_path(current_path, specifier)]
        }

        crate::source::Language::JavaScript
        | crate::source::Language::TypeScript
        | crate::source::Language::Tsx => {
            if specifier.starts_with('.') {
                let joined = current_parent.join(specifier);
                let Some(normalized) = normalize_import_path(&joined) else {
                    return Vec::new();
                };
                vec![PathBuf::from(normalized)]
            } else {
                return Vec::new();
            }
        }
    };

    let mut result = Vec::new();

    for raw in raw_paths {
        let normalized = raw.to_string_lossy().replace('\\', "/");

        result.push(normalized.clone());

        let extension = Path::new(&normalized).extension().and_then(|e| e.to_str());

        if extension.is_none() {
            let extensions: &[&str] = match language {
                crate::source::Language::Rust => &["rs"],
                crate::source::Language::Python => &["py"],
                crate::source::Language::JavaScript => &["js", "jsx"],
                crate::source::Language::TypeScript => &["ts", "tsx", "js", "jsx"],
                crate::source::Language::Tsx => &["tsx", "ts", "js", "jsx"],
            };

            for ext in extensions {
                result.push(format!("{normalized}.{ext}"));
                result.push(format!("{normalized}/index.{ext}"));
                result.push(format!("{normalized}/mod.{ext}"));
                result.push(format!("{normalized}/__init__.{ext}"));
            }
        }
    }

    result
}

fn normalize_import_path(path: &Path) -> Option<String> {
    use std::path::Component;

    let mut components = Vec::<String>::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                components.pop()?;
            }
            Component::Normal(value) => components.push(value.to_str()?.to_owned()),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if components.is_empty() {
        return None;
    }
    Some(components.join("/"))
}

fn rust_module_path(current_path: &str, specifier: &str) -> PathBuf {
    let mut segments = specifier.split("::").filter(|segment| !segment.is_empty());

    let first = segments.next().unwrap_or_default();
    let mut base = PathBuf::new();

    match first {
        "crate" => {
            if let Some(index) = Path::new(current_path)
                .components()
                .position(|component| component.as_os_str().to_string_lossy() == "src")
            {
                for component in Path::new(current_path).components().take(index + 1) {
                    base.push(component.as_os_str());
                }
            }
        }

        "self" => {
            base.push(
                Path::new(current_path)
                    .parent()
                    .unwrap_or_else(|| Path::new("")),
            );
        }

        "super" => {
            base.push(
                Path::new(current_path)
                    .parent()
                    .unwrap_or_else(|| Path::new("")),
            );

            base = base.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        }

        other => base.push(other),
    }

    for segment in segments {
        if matches!(segment, "self" | "super") {
            continue;
        }

        base.push(segment.trim_end_matches(";"));
    }

    base
}

fn python_module_path(current_path: &str, specifier: &str) -> PathBuf {
    let mut module = specifier;
    let mut dots = 0usize;

    while let Some(rest) = module.strip_prefix('.') {
        dots += 1;
        module = rest;
    }

    let mut base = PathBuf::from(
        Path::new(current_path)
            .parent()
            .unwrap_or_else(|| Path::new("")),
    );

    for _ in 1..dots {
        base = base.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
    }

    if module.is_empty() {
        return base;
    }

    for segment in module.split('.') {
        base.push(segment);
    }

    base
}

#[cfg(test)]
mod tests {
    use super::CoieEngine;
    use crate::{
        engines::coie::model::{EntityKind, RelationshipKind},
        source::SourceProvider,
    };
    use std::fs;

    #[test]
    fn normalizes_relative_imports_and_builds_edges_independent_of_file_order() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(
            root.path().join("src/a.ts"),
            "import { helper } from './lib/../b';\nexport function caller() { helper(); }\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("src/b.ts"),
            "export function helper() {}\n",
        )
        .unwrap();

        let snapshot = SourceProvider::default()
            .resolve(root.path(), "coie-relative-import-test")
            .unwrap();
        let graph = CoieEngine.extract(&snapshot).unwrap();

        assert!(graph.relationships.iter().any(|relationship| {
            relationship.kind == RelationshipKind::Imports
                && graph
                    .entities
                    .iter()
                    .any(|entity| entity.id == relationship.from && entity.path == "src/a.ts")
                && graph
                    .entities
                    .iter()
                    .any(|entity| entity.id == relationship.to && entity.path == "src/b.ts")
        }));
        assert!(graph.relationships.iter().any(|relationship| {
            relationship.kind == RelationshipKind::Calls
                && graph.entities.iter().any(|entity| {
                    entity.id == relationship.to
                        && entity.path == "src/b.ts"
                        && entity.name == "helper"
                })
        }));
    }

    #[test]
    fn extracts_imports_and_cross_file_calls_without_guessing_unimported_symbols() {
        let root = tempfile::tempdir().unwrap();

        fs::create_dir(root.path().join("src")).unwrap();

        fs::write(
            root.path().join("src/a.rs"),
            "use crate::b::helper;\nfn caller() { helper(); missing(); }\n",
        )
        .unwrap();

        fs::write(
            root.path().join("src/b.rs"),
            "pub fn helper() {}\npub fn missing() {}\n",
        )
        .unwrap();

        let snapshot = SourceProvider::default()
            .resolve(root.path(), "coie-engine-test")
            .unwrap();

        let graph = CoieEngine.extract(&snapshot).unwrap();

        let calls_helper = graph.relationships.iter().any(|relationship| {
            if relationship.kind != RelationshipKind::Calls {
                return false;
            }

            let Some(caller) = graph
                .entities
                .iter()
                .find(|entity| entity.id == relationship.from)
            else {
                return false;
            };

            let Some(target) = graph
                .entities
                .iter()
                .find(|entity| entity.id == relationship.to)
            else {
                return false;
            };

            caller.kind == EntityKind::Symbol
                && caller.name == "caller"
                && caller.path == "src/a.rs"
                && target.kind == EntityKind::Symbol
                && target.name == "helper"
                && target.path == "src/b.rs"
        });

        assert!(
            calls_helper,
            "expected caller() in src/a.rs to call imported helper() in src/b.rs"
        );

        assert!(
            graph
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "CALL_UNRESOLVED"),
            "expected missing() to remain unresolved instead of being guessed"
        );
    }
}

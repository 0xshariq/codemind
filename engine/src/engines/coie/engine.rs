use super::model::{
    CodeGraph, Diagnostic, EntityKind, GraphEntity, GraphRelationship, RelationshipKind,
};
use crate::engines::tie::TieEngine;
use crate::graph_storage::{GraphStorage, StorageError};
use crate::source::{SourceSnapshot, tree_sitter};
use std::{collections::HashMap, fs};

#[derive(Debug, Default, Clone, Copy)]
pub struct CoieEngine;

impl CoieEngine {
    pub fn extract(&self, snapshot: &SourceSnapshot) -> std::io::Result<CodeGraph> {
        let mut entities = Vec::new();
        let mut relationships = Vec::new();
        let mut diagnostics = Vec::new();
        let mut symbols_by_name = HashMap::<String, String>::new();

        for file in &snapshot.manifest.files {
            let Some(language) = file.language else {
                continue;
            };
            let source = fs::read_to_string(snapshot.root.join(&file.path))?;
            let file_id = entity_id(&snapshot.context.source_snapshot_id, &file.path, "file");
            entities.push(GraphEntity {
                id: file_id.clone(),
                kind: EntityKind::File,
                name: file.path.clone(),
                path: file.path.clone(),
                line: 1,
                context: snapshot.context.clone(),
            });
            let parsed = tree_sitter::parse(language, &source);
            if parsed.has_error {
                diagnostics.push(Diagnostic {
                    code: "PARSE_ERROR".to_owned(),
                    message: "Tree-sitter reported a syntax error; supported facts may be partial"
                        .to_owned(),
                    path: file.path.clone(),
                    severity: "warning".to_owned(),
                });
            }
            for (name, line) in parsed.declarations {
                let symbol_id = entity_id(
                    &snapshot.context.source_snapshot_id,
                    &format!("{}:{name}:{line}", file.path),
                    "symbol",
                );
                entities.push(GraphEntity {
                    id: symbol_id.clone(),
                    kind: EntityKind::Symbol,
                    name: name.clone(),
                    path: file.path.clone(),
                    line,
                    context: snapshot.context.clone(),
                });
                relationships.push(GraphRelationship {
                    id: edge_id(&file_id, &symbol_id, RelationshipKind::Defines),
                    kind: RelationshipKind::Defines,
                    from: file_id.clone(),
                    to: symbol_id.clone(),
                    context: snapshot.context.clone(),
                });
                symbols_by_name.insert(name, symbol_id);
            }
            for import in parsed.imports {
                diagnostics.push(Diagnostic {
                    code: "IMPORT_UNRESOLVED".to_owned(),
                    message: format!(
                        "Import `{import}` retained as diagnostic until path resolution is available"
                    ),
                    path: file.path.clone(),
                    severity: "info".to_owned(),
                });
            }
            for name in parsed.calls {
                let short_name = name.rsplit('.').next().unwrap_or(&name);
                if let Some(target) = symbols_by_name.get(short_name) {
                    relationships.push(GraphRelationship {
                        id: edge_id(&file_id, target, RelationshipKind::Calls),
                        kind: RelationshipKind::Calls,
                        from: file_id.clone(),
                        to: target.clone(),
                        context: snapshot.context.clone(),
                    });
                }
            }
        }
        Ok(CodeGraph {
            context: snapshot.context.clone(),
            entities,
            relationships,
            diagnostics,
        })
    }

    pub async fn ingest<S: GraphStorage>(
        &self,
        snapshot: &SourceSnapshot,
        storage: &S,
    ) -> Result<CodeGraph, StorageError> {
        let graph = self
            .extract(snapshot)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        storage.write_code_graph(&graph).await?;
        let (tests, relationships) = TieEngine
            .discover(snapshot)
            .map_err(|error| StorageError::Query(error.to_string()))?;
        storage.write_test_graph(&tests, &relationships).await?;
        Ok(graph)
    }
}

fn entity_id(snapshot: &str, value: &str, kind: &str) -> String {
    format!("{kind}:{snapshot}:{:x}", stable_hash(value))
}

fn edge_id(from: &str, to: &str, kind: RelationshipKind) -> String {
    format!("edge:{:?}:{:x}", kind, stable_hash(&format!("{from}:{to}")))
}

fn stable_hash(value: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

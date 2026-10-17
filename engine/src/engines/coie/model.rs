use crate::source::{Language, RepositoryContext, tree_sitter::SourceRange};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntityKind {
    Repository,
    RepositorySnapshot,
    Directory,
    File,
    Module,
    Package,
    Symbol,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationshipKind {
    Contains,
    Defines,
    Imports,
    Calls,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResolutionStatus {
    Resolved,
    Unresolved,
    Ambiguous,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceEvidence {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub snapshot_id: String,
    pub provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub path: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphEntity {
    pub id: String,
    pub kind: EntityKind,
    pub name: String,
    pub path: String,
    pub line: u32,
    pub context: RepositoryContext,
    pub language: Option<Language>,
    pub native_kind: Option<String>,
    pub range: Option<SourceRange>,
    pub resolution: ResolutionStatus,
    pub evidence: Option<SourceEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphRelationship {
    pub id: String,
    pub kind: RelationshipKind,
    pub from: String,
    pub to: String,
    pub context: RepositoryContext,
    pub resolution: ResolutionStatus,
    pub evidence: Option<SourceEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeGraph {
    pub context: RepositoryContext,
    pub entities: Vec<GraphEntity>,
    pub relationships: Vec<GraphRelationship>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphBuildReport {
    pub context: RepositoryContext,
    pub entity_count: usize,
    pub relationship_count: usize,
    pub diagnostic_count: usize,
    pub warning_count: usize,
}

impl GraphBuildReport {
    pub fn from_graph(graph: &CodeGraph) -> Self {
        Self {
            context: graph.context.clone(),
            entity_count: graph.entities.len(),
            relationship_count: graph.relationships.len(),
            diagnostic_count: graph.diagnostics.len(),
            warning_count: graph
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity.eq_ignore_ascii_case("warning"))
                .count(),
        }
    }
}

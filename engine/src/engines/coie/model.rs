use crate::source::RepositoryContext;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntityKind {
    File,
    Symbol,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationshipKind {
    Defines,
    Imports,
    Calls,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphRelationship {
    pub id: String,
    pub kind: RelationshipKind,
    pub from: String,
    pub to: String,
    pub context: RepositoryContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeGraph {
    pub context: RepositoryContext,
    pub entities: Vec<GraphEntity>,
    pub relationships: Vec<GraphRelationship>,
    pub diagnostics: Vec<Diagnostic>,
}

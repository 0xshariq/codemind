use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositoryContext {
    pub repository_id: String,
    pub workspace_id: String,
    pub source_snapshot_id: String,
    pub graph_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub root: std::path::PathBuf,
    pub context: RepositoryContext,
    pub manifest: super::SourceManifest,
}

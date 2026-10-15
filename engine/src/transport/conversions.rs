use crate::contract::generated::RepositoryContext;
use crate::source;

pub fn context(value: &RepositoryContext) -> source::RepositoryContext {
    source::RepositoryContext {
        repository_id: value.repository_id.clone(),
        workspace_id: value.workspace_id.clone(),
        source_snapshot_id: value.source_snapshot_id.clone(),
        graph_version: value.graph_version.clone(),
    }
}

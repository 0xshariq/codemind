use crate::{
    contract::result::{CanonicalResult, Completeness, Correlation, Freshness, Outcome},
    source::RepositoryContext,
};
use serde::{Deserialize, Serialize};

/// The hackathon history boundary is source-state provenance, not a full Git
/// history database. This value identifies exactly which repository snapshot
/// and graph version the current analysis belongs to.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotProvenance {
    pub repository_id: String,
    pub workspace_id: String,
    pub source_snapshot_id: String,
    pub graph_version: String,
}

/// Return the only history fact required by the frozen hackathon MVP: the
/// analyzed source state and its graph version. Full commit/PR/developer
/// history is intentionally outside this query boundary.
pub fn snapshot_provenance(context: &RepositoryContext) -> CanonicalResult<SnapshotProvenance> {
    let data = SnapshotProvenance {
        repository_id: context.repository_id.clone(),
        workspace_id: context.workspace_id.clone(),
        source_snapshot_id: context.source_snapshot_id.clone(),
        graph_version: context.graph_version.clone(),
    };

    CanonicalResult::new(
        context.clone(),
        Correlation {
            request_id: "coie-history-provenance".into(),
            trace_id: "coie-history-provenance".into(),
            operation_id: "coie-history-provenance".into(),
        },
        Outcome::Success,
        Completeness::Complete,
        Freshness::Current,
        data,
    )
}

/// Two history queries are compatible only when they describe the same
/// repository, workspace, source snapshot and graph version.
pub fn compatible(left: &RepositoryContext, right: &RepositoryContext) -> bool {
    left == right
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(snapshot: &str, graph: &str) -> RepositoryContext {
        RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: snapshot.into(),
            graph_version: graph.into(),
        }
    }

    #[test]
    fn provenance_identifies_the_exact_analyzed_source_state() {
        let context = context("snapshot-a", "graph-a");
        let result = snapshot_provenance(&context);
        assert!(matches!(result.outcome, Outcome::Success));
        assert_eq!(result.data.source_snapshot_id, "snapshot-a");
        assert_eq!(result.data.graph_version, "graph-a");
    }

    #[test]
    fn provenance_compatibility_rejects_mixed_source_states() {
        let first = context("snapshot-a", "graph-a");
        let second = context("snapshot-b", "graph-b");
        assert!(compatible(&first, &first));
        assert!(!compatible(&first, &second));
    }
}

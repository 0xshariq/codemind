use crate::{
    contract::result::{
        CanonicalResult, Completeness, Correlation, Freshness, MAX_RELATIONSHIPS, Outcome,
    },
    engines::coie::model::{GraphRelationship, RelationshipKind},
    source::RepositoryContext,
};

pub fn of_kind(
    relationships: &[GraphRelationship],
    kind: RelationshipKind,
) -> impl Iterator<Item = &GraphRelationship> {
    relationships.iter().filter(move |r| r.kind == kind)
}

pub fn bounded_for_context(
    relationships: &[GraphRelationship],
    context: &RepositoryContext,
    limit: usize,
) -> CanonicalResult<Vec<GraphRelationship>> {
    let mut matching: Vec<_> = relationships
        .iter()
        .filter(|r| &r.context == context)
        .cloned()
        .collect();
    let foreign = relationships.len().saturating_sub(matching.len());
    let limit = limit.min(MAX_RELATIONSHIPS);
    let truncated = matching.len() > limit;
    matching.truncate(limit);
    let mut result = CanonicalResult::new(
        context.clone(),
        Correlation {
            request_id: "internal-relationship-query".into(),
            trace_id: "internal-relationship-query".into(),
            operation_id: "internal-relationship-query".into(),
        },
        if matching.is_empty() {
            Outcome::Empty
        } else if truncated {
            Outcome::Partial
        } else {
            Outcome::Success
        },
        if truncated {
            Completeness::Truncated
        } else {
            Completeness::Complete
        },
        Freshness::Current,
        matching,
    );
    if foreign > 0 {
        result.diagnostics.push(format!(
            "{foreign} relationships from incompatible context were excluded"
        ));
    }
    if truncated {
        result
            .limitations
            .push("relationship result was truncated by the requested limit".into());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::coie::model::{GraphRelationship, ResolutionStatus};

    fn context() -> crate::source::RepositoryContext {
        crate::source::RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn relationship_queries_filter_kind_and_context_and_report_foreign_data() {
        let current = GraphRelationship {
            id: "r".into(),
            kind: RelationshipKind::Calls,
            from: "a".into(),
            to: "b".into(),
            context: context(),
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        };
        let mut foreign = current.clone();
        foreign.id = "foreign".into();
        foreign.context.workspace_id = "other".into();
        assert_eq!(
            of_kind(std::slice::from_ref(&current), RelationshipKind::Calls).count(),
            1
        );
        let result = bounded_for_context(&[current, foreign], &context(), 10);
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.diagnostics.len(), 1);
    }
}

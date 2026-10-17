use crate::{
    contract::result::{
        CanonicalResult, Completeness, Correlation, Freshness, MAX_RESULT_ITEMS, Outcome,
    },
    engines::coie::model::GraphEntity,
    source::RepositoryContext,
};

pub fn by_name<'a>(entities: &'a [GraphEntity], name: &str) -> Vec<&'a GraphEntity> {
    entities.iter().filter(|e| e.name == name).collect()
}

pub fn bounded_by_name(
    entities: &[GraphEntity],
    context: &RepositoryContext,
    name: &str,
    limit: usize,
) -> CanonicalResult<Vec<GraphEntity>> {
    let mut data: Vec<_> = entities
        .iter()
        .filter(|e| e.context == *context && e.name == name)
        .cloned()
        .collect();
    let limit = limit.min(MAX_RESULT_ITEMS);
    let truncated = data.len() > limit;
    data.truncate(limit);
    CanonicalResult::new(
        context.clone(),
        Correlation {
            request_id: "internal-symbol-search".into(),
            trace_id: "internal-symbol-search".into(),
            operation_id: "internal-symbol-search".into(),
        },
        if data.is_empty() {
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
        data,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::coie::model::{EntityKind, GraphEntity, ResolutionStatus};

    fn context() -> crate::source::RepositoryContext {
        crate::source::RepositoryContext {
            repository_id: "repo".into(),
            workspace_id: "workspace".into(),
            source_snapshot_id: "snapshot".into(),
            graph_version: "graph".into(),
        }
    }

    #[test]
    fn search_respects_context_and_result_bounds() {
        let current = GraphEntity {
            id: "a".into(),
            kind: EntityKind::Symbol,
            name: "target".into(),
            path: "src/lib.rs".into(),
            line: 1,
            context: context(),
            language: None,
            native_kind: Some("function".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        };
        let mut foreign = current.clone();
        foreign.id = "b".into();
        foreign.context.workspace_id = "other".into();
        assert_eq!(by_name(&[current.clone(), foreign], "target").len(), 2);
        assert_eq!(
            bounded_by_name(&[current], &context(), "target", 1)
                .data
                .len(),
            1
        );
    }
}

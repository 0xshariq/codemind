use crate::{
    contract::result::{
        CanonicalResult, Completeness, Correlation, Freshness, MAX_RESULT_ITEMS, Outcome,
    },
    engines::coie::model::{EntityKind, GraphEntity},
    source::RepositoryContext,
};

pub fn symbols(entities: &[GraphEntity]) -> impl Iterator<Item = &GraphEntity> {
    entities.iter().filter(|e| e.kind == EntityKind::Symbol)
}

pub fn bounded_symbols(
    entities: &[GraphEntity],
    context: &RepositoryContext,
    limit: usize,
) -> CanonicalResult<Vec<GraphEntity>> {
    let mut data: Vec<_> = symbols(entities)
        .filter(|e| e.context == *context)
        .cloned()
        .collect();
    let limit = limit.min(MAX_RESULT_ITEMS);
    let truncated = data.len() > limit;
    data.truncate(limit);
    CanonicalResult::new(
        context.clone(),
        Correlation {
            request_id: "internal-symbol-query".into(),
            trace_id: "internal-symbol-query".into(),
            operation_id: "internal-symbol-query".into(),
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
    fn symbol_query_excludes_files_and_preserves_context_bounds() {
        let symbol = GraphEntity {
            id: "symbol".into(),
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
        let file = GraphEntity {
            id: "file".into(),
            kind: EntityKind::File,
            name: "src/lib.rs".into(),
            path: "src/lib.rs".into(),
            line: 1,
            context: context(),
            language: None,
            native_kind: Some("file".into()),
            range: None,
            resolution: ResolutionStatus::Resolved,
            evidence: None,
        };
        assert_eq!(symbols(&[symbol.clone(), file]).count(), 1);
        assert_eq!(bounded_symbols(&[symbol], &context(), 10).data.len(), 1);
    }
}

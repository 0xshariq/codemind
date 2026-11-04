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

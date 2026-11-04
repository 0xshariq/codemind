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

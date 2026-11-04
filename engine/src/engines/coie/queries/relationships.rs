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

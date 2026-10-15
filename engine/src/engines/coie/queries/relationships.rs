use crate::contract::result::{CanonicalResult, Completeness, Freshness, Outcome};
use crate::engines::coie::model::{GraphRelationship, RelationshipKind};
use crate::source::RepositoryContext;

pub fn of_kind(
    relationships: &[GraphRelationship],
    kind: RelationshipKind,
) -> impl Iterator<Item = &GraphRelationship> {
    relationships
        .iter()
        .filter(move |relationship| relationship.kind == kind)
}

pub fn bounded_for_context(
    relationships: &[GraphRelationship],
    context: &RepositoryContext,
    limit: usize,
) -> CanonicalResult<Vec<GraphRelationship>> {
    let mut diagnostics = Vec::new();
    let mut matching: Vec<_> = relationships
        .iter()
        .filter(|relationship| &relationship.context == context)
        .cloned()
        .collect();
    if relationships
        .iter()
        .any(|relationship| &relationship.context != context)
    {
        diagnostics.push("relationships from incompatible context were excluded".to_owned());
    }
    let limit = limit.min(crate::contract::result::MAX_RELATIONSHIPS);
    let truncated = matching.len() > limit;
    matching.truncate(limit);
    CanonicalResult {
        outcome: if matching.is_empty() {
            Outcome::Empty
        } else if truncated {
            Outcome::Partial
        } else {
            Outcome::Success
        },
        completeness: if truncated {
            Completeness::Truncated
        } else {
            Completeness::Complete
        },
        freshness: Freshness::Current,
        context: context.clone(),
        data: matching,
        evidence: Vec::new(),
        diagnostics,
        limitations: if truncated {
            vec!["relationship result was truncated by the requested limit".to_owned()]
        } else {
            Vec::new()
        },
    }
}

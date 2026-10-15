pub const RELATIONSHIP_QUERY_LIMIT: usize = 1_000;

pub fn bounded_limit(limit: usize) -> usize {
    limit.clamp(1, RELATIONSHIP_QUERY_LIMIT)
}

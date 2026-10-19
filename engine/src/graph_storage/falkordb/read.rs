pub const RELATIONSHIP_QUERY_LIMIT: usize = 1_000;
pub const SEARCH_QUERY_LIMIT: usize = 101;
pub const PATH_QUERY_LIMIT: usize = 50;
pub const PATH_DEPTH_LIMIT: usize = 8;

pub fn bounded_limit(limit: usize) -> usize {
    limit.clamp(1, RELATIONSHIP_QUERY_LIMIT)
}

pub fn bounded_search_limit(limit: usize) -> usize {
    limit.clamp(1, SEARCH_QUERY_LIMIT)
}

pub fn bounded_path(max_depth: usize, max_paths: usize) -> (usize, usize) {
    (
        max_depth.clamp(1, PATH_DEPTH_LIMIT),
        max_paths.clamp(1, PATH_QUERY_LIMIT),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_bounds_are_hard_capped_and_never_zero() {
        assert_eq!(bounded_limit(0), 1);
        assert_eq!(bounded_limit(usize::MAX), RELATIONSHIP_QUERY_LIMIT);
        assert_eq!(bounded_search_limit(0), 1);
        assert_eq!(bounded_search_limit(usize::MAX), SEARCH_QUERY_LIMIT);
        assert_eq!(bounded_path(0, 0), (1, 1));
        assert_eq!(
            bounded_path(usize::MAX, usize::MAX),
            (PATH_DEPTH_LIMIT, PATH_QUERY_LIMIT)
        );
    }
}

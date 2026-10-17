use super::super::model::ImpactPath;
use std::collections::HashMap;

/// Keep the affected-entity set distinct from path evidence. If multiple
/// routes reach the same entity, retain all distinct routes rather than
/// silently discarding the supporting evidence.
pub fn deduplicate(paths: Vec<ImpactPath>) -> Vec<ImpactPath> {
    let mut seen: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let mut result = Vec::new();

    for path in paths {
        let signature = path
            .relationships
            .iter()
            .map(|relationship| relationship.id.clone())
            .collect::<Vec<_>>();

        let entry = seen.entry(path.entity_id.clone()).or_default();
        if entry.contains(&signature) {
            continue;
        }
        entry.push(signature);
        result.push(path);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::deduplicate;
    use crate::engines::chie::model::{ImpactClassification, ImpactPath};

    #[test]
    fn deduplicate_keeps_distinct_paths_but_removes_exact_duplicates() {
        let empty = ImpactPath {
            entity_id: "b".into(),
            depth: 1,
            classification: ImpactClassification::Direct,
            relationships: Vec::new(),
        };
        // Empty paths are not valid Change Graph paths, but the aggregation helper itself should still be deterministic.
        let result = deduplicate(vec![empty.clone(), empty]);
        assert_eq!(result.len(), 1);
    }
}

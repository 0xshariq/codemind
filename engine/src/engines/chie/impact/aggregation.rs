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

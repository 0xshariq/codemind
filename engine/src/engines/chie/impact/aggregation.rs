use super::super::model::ImpactPath;

pub fn deduplicate(paths: Vec<ImpactPath>) -> Vec<ImpactPath> {
    let mut result = Vec::new();
    for path in paths {
        if result
            .iter()
            .all(|existing: &ImpactPath| existing.entity_id != path.entity_id)
        {
            result.push(path);
        }
    }
    result
}
